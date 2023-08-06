use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand, ProxyCommand},
    player::components::{Character, Client, Online},
    spatial::components::{Position, Tile, Transition, Zone},
    value_or_continue,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_enter(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^enter( (?P<transition>.+))?").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let transition = captures
                .name("transition")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Enter(transition))
        }
    }
}

pub fn enter(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut proxy: EventWriter<ProxyCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(Entity, &Client, &Character, &Parent), With<Online>>,
    transitions: Query<(&Transition, &Depiction)>,
    tiles: Query<(Entity, &Position, &Parent, Option<&Children>), With<Tile>>,
    zones: Query<&Zone>,
) {
    for command in commands.iter() {
        if let Command::Enter(target) = &command.command {
            let (player, client, character, tile) =
                value_or_continue!(players.iter_mut().find(|(_, c, _, _)| c.id == command.from));

            if character.state.is_combat() {
                outbox.send_text(client.id, "You can't move while in combat.");

                continue;
            }

            let (_, _, _, siblings) = value_or_continue!(tiles.get(tile.get()).ok());

            let transitions = siblings
                .map(|siblings| {
                    siblings
                        .iter()
                        .filter_map(|child| transitions.get(*child).ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(Vec::new);

            if transitions.is_empty() {
                outbox.send_text(client.id, "There is nowhere to enter from here.");

                continue;
            }

            let Some((transition, _)) = transitions.iter().find(|(_, depiction)| {
                target
                    .as_ref()
                    .map_or(true, |tag| depiction.tags.contains(tag))
            }) else {
                outbox.send_text(client.id, "Could not find entrance.");

                continue;
            };

            let target = value_or_continue!(tiles.iter().find_map(|(e, p, z, _)| {
                zones.get(z.get()).ok().and_then(|zone| {
                    if zone.name == transition.zone && p.0 == transition.position {
                        Some(e)
                    } else {
                        None
                    }
                })
            }));

            bevy.entity(player).set_parent(target);

            proxy.send(ProxyCommand(ParsedCommand {
                from: client.id,
                command: Command::Look(None),
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
        transition_builder::TransitionBuilder,
        utils::{get_message_content, send_message},
    };

    use super::*;

    #[test]
    fn parses() {
        let target = handle_enter("enter the void");
        assert_eq!(target, Ok(Command::Enter(Some("the void".into()))));

        let no_target = handle_enter("enter");
        assert_eq!(no_target, Ok(Command::Enter(None)));
    }

    #[test]
    fn enters_by_tag() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, enter);

        let start_zone = ZoneBuilder::new().build(&mut app);
        let start = TileBuilder::new().build(&mut app, start_zone);

        let destination_zone = ZoneBuilder::new().build(&mut app);
        let destination = TileBuilder::new().build(&mut app, destination_zone);

        TransitionBuilder::new()
            .tags(&vec!["the void"])
            .build(&mut app, start, destination);

        let (player, client_id, _) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "enter the void");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), destination);
    }

    #[test]
    fn enters_first_if_no_tag() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, enter);

        let start_zone = ZoneBuilder::new().build(&mut app);
        let start = TileBuilder::new().build(&mut app, start_zone);

        let first_zone = ZoneBuilder::new().build(&mut app);
        let first = TileBuilder::new().build(&mut app, first_zone);

        let second_zone = ZoneBuilder::new().build(&mut app);
        let second = TileBuilder::new().build(&mut app, second_zone);

        TransitionBuilder::new().build(&mut app, start, first);
        TransitionBuilder::new().build(&mut app, start, second);

        let (player, client_id, _) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "enter");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), first);
    }

    #[test]
    fn transition_not_found() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, enter);

        let start_zone = ZoneBuilder::new().build(&mut app);
        let start = TileBuilder::new().build(&mut app, start_zone);

        let other_zone = ZoneBuilder::new().build(&mut app);
        let other = TileBuilder::new().build(&mut app, other_zone);

        TransitionBuilder::new()
            .tags(&vec!["enter the void"])
            .build(&mut app, start, other);

        let (_, client_id, _) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "enter at your own risk");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert!(content.contains("Could not find entrance."));
    }

    #[test]
    fn no_transition() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, enter);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "enter the dragon");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert!(content.contains("There is nowhere to enter from here."));
    }
}
