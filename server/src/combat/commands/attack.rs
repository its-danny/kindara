use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    combat::components::{Attributes, HasAttacked, HitError, InCombat, QueuedAttack, State},
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    mastery::resources::Masteries,
    npc::components::Npc,
    player::{
        components::{Character, Client, Online},
        events::Prompt,
    },
    skills::resources::{Skill, Skills},
    spatial::components::Tile,
    value_or_continue,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_attack(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(?P<skill>.*?)( (?P<target>.*?))?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let skill = captures
                .name("skill")
                .map(|m| m.as_str().trim().to_lowercase())
                .ok_or(ParseError::InvalidArguments("What skill?".into()))?;

            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Attack((skill, target)))
        }
    }
}

pub fn attack(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut npcs: Query<
        (
            Entity,
            &Depiction,
            Option<&mut State>,
            Option<&Interactions>,
        ),
        With<Npc>,
    >,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<
        (
            Entity,
            &mut Character,
            &Attributes,
            &Client,
            &Parent,
            Option<&InCombat>,
            Option<&HasAttacked>,
            Option<&mut QueuedAttack>,
        ),
        With<Online>,
    >,
    mut prompts: EventWriter<Prompt>,
    skills: Res<Skills>,
    masteries: Res<Masteries>,
    tiles: Query<&Children, With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Attack((skill, target)) = &command.command {
            let mut first_attack: Option<InCombat> = None;

            let (
                player,
                character,
                attributes,
                client,
                tile,
                mut in_combat,
                has_attacked,
                queued_attack,
            ) = value_or_continue!(players
                .iter_mut()
                .find(|(_, _, _, c, _, _, _, _)| c.id == command.from));

            let skill = match get_skill(&skills, &masteries, &character, skill) {
                Ok(skill) => skill,
                Err(e) => {
                    outbox.send_text(client.id, e.to_string());

                    continue;
                }
            };

            if let Some(target) = target {
                let siblings = value_or_continue!(tiles.get(tile.get()).ok());

                let Some((entity, _, _, interactions)) = siblings
                    .iter()
                    .filter_map(|sibling| npcs.get(*sibling).ok())
                    .find(|(entity, depiction, _, _)| depiction.matches_query(entity, target))
                else {
                    outbox.send_text(client.id, format!("You don't see a {target} here."));

                    continue;
                };

                if !interactions.map_or(false, |i| i.0.contains(&Interaction::Attack)) {
                    outbox.send_text(client.id, format!("You can't attack the {target}."));

                    continue;
                }

                first_attack = Some(InCombat {
                    target: entity,
                    distance: skill.distance,
                });

                bevy.entity(player).insert(InCombat {
                    target: entity,
                    distance: skill.distance,
                });

                bevy.entity(entity).insert(InCombat {
                    target: player,
                    distance: skill.distance,
                });
            }

            if first_attack.is_some() {
                in_combat = first_attack.as_ref();
            }

            let Some(in_combat) = in_combat else {
                outbox.send_text(client.id, "You are not in combat.");

                continue;
            };

            let (_, depiction, state, _) = value_or_continue!(npcs.get_mut(in_combat.target).ok());

            let Some(mut state) = state else {
                outbox.send_text(
                    client.id,
                    format!("You can't attack the {}.", depiction.short_name),
                );

                continue;
            };

            if has_attacked.is_some() {
                match queued_attack {
                    Some(mut queued_attack) => {
                        queued_attack.0 = command.clone();

                        outbox.send_text(client.id, "Queued attack replaced.");
                    }
                    None => {
                        bevy.entity(player).insert(QueuedAttack(command.clone()));

                        outbox.send_text(client.id, "Attack queued.");
                    }
                }

                continue;
            }

            match in_combat.attack(&mut bevy, player, skill, attributes, &mut state) {
                Ok(_) => {
                    outbox.send_text(
                        client.id,
                        format!(
                            "You attack the {}. It's health is now {}.",
                            depiction.short_name, state.health
                        ),
                    );
                }
                Err(HitError::Missed) => {
                    outbox.send_text(
                        client.id,
                        format!("You attack the {} but miss.", depiction.short_name),
                    );
                }
            }

            prompts.send(Prompt::new(client.id));
        }
    }
}

#[derive(Error, Debug)]
enum SkillError {
    #[error("You don't know how to do that.")]
    UnknownSkill,
}

fn get_skill<'a>(
    skills: &'a Res<'a, Skills>,
    masteries: &Res<Masteries>,
    character: &Character,
    skill: &String,
) -> Result<&'a Skill, SkillError> {
    masteries
        .0
        .get(&character.mastery)
        .filter(|mastery| mastery.skills.contains(skill))
        .and_then(|_| skills.0.get(skill))
        .ok_or(SkillError::UnknownSkill)
}

#[cfg(test)]
mod tests {
    use crate::test::{
        app_builder::AppBuilder,
        npc_builder::NpcBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
        utils::{get_message_content, send_message},
    };

    use super::*;

    #[test]
    fn parses() {
        let target = handle_attack("fireball goat");
        assert_eq!(
            target,
            Ok(Command::Attack(("fireball".into(), Some("goat".into()))))
        );

        let no_target = handle_attack("fireball");
        assert_eq!(no_target, Ok(Command::Attack(("fireball".into(), None))));
    }

    #[test]
    fn valid_target() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let npc = NpcBuilder::new()
            .name("Pazuzu")
            .interactions(vec![Interaction::Attack])
            .tile(tile)
            .combat(true)
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch pazuzu");
        app.update();

        assert!(app.world.get::<InCombat>(player).is_some());
        assert!(app.world.get::<InCombat>(npc).is_some());
    }

    #[test]
    fn existing_target() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        NpcBuilder::new()
            .name("Pazuzu")
            .interactions(vec![Interaction::Attack])
            .tile(tile)
            .combat(true)
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch pazuzu");
        app.update();

        send_message(&mut app, client_id, "punch");
        app.update();

        assert!(app.world.get::<InCombat>(player).is_some());
    }

    #[test]
    fn invalid_target() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        NpcBuilder::new().name("Pazuzu").tile(tile).build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch pazuzu");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You can't attack the pazuzu.");
        assert!(app.world.get::<InCombat>(player).is_none());
    }
}
