use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use caith::Roller;
use regex::Regex;

use crate::{
    combat::components::{Attributes, HasAttacked, QueuedAttack, State},
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    mastery::resources::Masteries,
    npc::components::Npc,
    player::{
        components::{Character, CharacterState, Client, Online},
        events::Prompt,
    },
    skills::resources::{Action, RelevantStat, Skills},
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
            let (player, mut character, attributes, client, tile, has_attacked, queued_attack) =
                value_or_continue!(players
                    .iter_mut()
                    .find(|(_, _, _, c, _, _, _)| c.id == command.from));

            let mastery = value_or_continue!(masteries.0.get(&character.mastery));

            if !mastery.skills.contains(skill) {
                outbox.send_text(client.id, "You don't know how to do that.");

                continue;
            }

            let Some(skill) = skills.0.get(skill.as_str()) else {
                outbox.send_text(client.id, "You don't know how to do that.");

                continue;
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

                if interactions.map_or(true, |i| !i.0.contains(&Interaction::Attack)) {
                    outbox.send_text(client.id, format!("You can't attack the {target}."));

                    continue;
                }

                character.state = CharacterState::Combat(entity);
            }

            let CharacterState::Combat(entity) = character.state else {
                outbox.send_text(client.id, "You are not in combat.");

                continue;
            };

            let (_, depiction, state, _) = value_or_continue!(npcs.get_mut(entity).ok());

            let Some(mut state) = state else {
                outbox.send_text(
                    client.id,
                    format!("You can't attack the {}.", depiction.short_name),
                );

                continue;
            };

            // If they've already attacked, queue the attack
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

            // Roll for attack quality
            let roller = Roller::new("2d10").unwrap();
            let quality = roller.roll().unwrap();
            let quality = quality.as_single().unwrap();

            // Roll enemy dodge
            let roller = Roller::new("1d8").unwrap();
            let dodge = roller.roll().unwrap();
            let dodge = dodge.as_single().unwrap();

            // Check if attack hits
            if quality.get_total() < dodge.get_total() {
                outbox.send_text(
                    client.id,
                    format!("You attack the {}, but miss.", depiction.short_name),
                );

                continue;
            }

            // TODO: Roll resistances

            // Apply skill actions
            for action in &skill.actions {
                match action {
                    Action::ApplyDamage(roll) => {
                        let roller = Roller::new(roll).unwrap();
                        let damage = roller.roll().unwrap();
                        let result = damage.as_single().unwrap();
                        let mut damage = result.get_total() as i32;

                        match &skill.stat {
                            RelevantStat::Strength => {
                                damage += attributes.strength as i32;
                            }
                            RelevantStat::Dexterity => {
                                damage += attributes.dexterity as i32;
                            }
                            RelevantStat::Intelligence => {
                                damage += attributes.intelligence as i32;
                            }
                        }

                        state.apply_damage(damage);
                    }
                }
            }

            bevy.entity(player).insert(HasAttacked {
                timer: Timer::from_seconds(attributes.speed as f32, TimerMode::Once),
            });

            outbox.send_text(
                client.id,
                format!(
                    "You attack the {}. It's health is now {}.",
                    depiction.short_name, state.health
                ),
            );

            prompts.send(Prompt::new(client.id));
        }
    }
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

        NpcBuilder::new()
            .name("Pazuzu")
            .interactions(vec![Interaction::Attack])
            .tile(tile)
            .combat(true)
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch pazuzu");
        app.update();

        let character = app.world.get::<Character>(player).unwrap();

        assert!(character.state.is_combat());
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

        let character = app.world.get::<Character>(player).unwrap();

        assert!(character.state.is_combat());
    }

    #[test]
    fn invalid_target() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        NpcBuilder::new().name("Pazuzu").tile(tile).build(&mut app);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch pazuzu");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You can't attack the pazuzu.");
    }
}
