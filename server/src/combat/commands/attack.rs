use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    combat::components::{Attributes, HasAttacked, State},
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    npc::components::Npc,
    player::{
        components::{Character, CharacterState, Client, Online},
        events::Prompt,
    },
    skills::resources::{Action, Skills},
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
        ),
        With<Online>,
    >,
    mut prompts: EventWriter<Prompt>,
    skills: Res<Skills>,
    tiles: Query<&Children, With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Attack((skill, target)) = &command.command {
            let (player, mut character, attributes, client, tile, has_attacked) =
                value_or_continue!(players
                    .iter_mut()
                    .find(|(_, _, _, c, _, _)| c.id == command.from));

            let Some(skill) = skills.0.get(skill.as_str()) else {
                outbox.send_text(client.id, "You don't know how to do that.");

                continue;
            };

            if has_attacked.is_some() {
                outbox.send_text(client.id, "You're not ready to attack again.");

                continue;
            }

            if let Some(target) = target {
                let siblings = value_or_continue!(tiles.get(tile.get()).ok());

                let Some((entity, _, _,interactions)) = siblings
                    .iter()
                    .filter_map(|sibling| npcs.get(*sibling).ok())
                    .find(|(entity, depiction,_, _)| depiction.matches_query(entity, target)) else {
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
                    outbox.send_text(client.id, format!("You can't attack the {}.", depiction.short_name));

                    continue;
            };

            for action in &skill.actions {
                match action {
                    Action::ApplyDamage(damage) => {
                        state.apply_damage(*damage);
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
    fn valid_target() {
        let mut app = AppBuilder::new().build();
        app.add_system(attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        NpcBuilder::new()
            .name("Pazuzu")
            .interactions(vec![Interaction::Attack])
            .tile(tile)
            .combat(true)
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "attack pazuzu");
        app.update();

        let character = app.world.get::<Character>(player).unwrap();

        assert!(character.state.is_combat());
    }

    #[test]
    fn existing_target() {
        let mut app = AppBuilder::new().build();
        app.add_system(attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        NpcBuilder::new()
            .name("Pazuzu")
            .interactions(vec![Interaction::Attack])
            .tile(tile)
            .combat(true)
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "attack pazuzu");
        app.update();

        send_message(&mut app, client_id, "attack");
        app.update();

        let character = app.world.get::<Character>(player).unwrap();

        assert!(character.state.is_combat());
    }

    #[test]
    fn invalid_target() {
        let mut app = AppBuilder::new().build();
        app.add_system(attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        NpcBuilder::new().name("Pazuzu").tile(tile).build(&mut app);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "attack pazuzu");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You can't attack the pazuzu.");
    }
}
