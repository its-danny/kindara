use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Client, Online},
    spatial::components::{Door, Tile},
    value_or_continue,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_open(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^open( (?P<target>.+))?").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Open(target))
        }
    }
}

pub fn open(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Parent), With<Online>>,
    tiles: Query<Option<&Children>, With<Tile>>,
    doors: Query<(Entity, &Depiction), With<Door>>,
    mut doors_mut: Query<&mut Door>,
) {
    for command in commands.iter() {
        if let Command::Open(target) = &command.command {
            let (client, tile) =
                value_or_continue!(players.iter().find(|(c, _)| c.id == command.from));

            let siblings = value_or_continue!(tiles.get(tile.get()).ok());

            let Some(target) = target else {
                outbox.send_text(client.id, "Open what?");

                continue;
            };

            let Some((entity, _)) = siblings
                .iter()
                .flat_map(|siblings| siblings.iter())
                .filter_map(|sibling| doors.get(*sibling).ok())
                .find(|(entity, depiction)| depiction.matches_query(entity, target)) else {
                    outbox.send_text(client.id, "You can't open that.");

                    continue;
            };

            let Ok(mut door) = doors_mut.get_mut(entity) else {
                outbox.send_text(client.id, "You can't open that.");

                continue;
            };

            door.is_open = true;

            outbox.send_text(client.id, "You open the door.");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::{
        app_builder::AppBuilder,
        item_builder::ItemBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
        utils::{get_message_content, send_message},
    };

    use super::*;

    #[test]
    fn parses() {
        let target = handle_open("open door");
        assert_eq!(target, Ok(Command::Open(Some("door".into()))));

        let no_target = handle_open("open");
        assert_eq!(no_target, Ok(Command::Open(None)));
    }

    #[test]
    fn opens_door() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, open);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let door = ItemBuilder::new().name("door").tile(tile).build(&mut app);
        app.world.entity_mut(door).insert(Door {
            is_open: false,
            blocks: IVec3::new(0, -1, 0),
        });

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "open door");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You open the door.");

        assert!(app.world.get::<Door>(door).unwrap().is_open);
    }
}