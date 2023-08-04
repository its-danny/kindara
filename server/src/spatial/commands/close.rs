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

pub fn handle_close(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^close( (?P<target>.+))?").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Close(target))
        }
    }
}

pub fn close(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Parent), With<Online>>,
    tiles: Query<Option<&Children>, With<Tile>>,
    doors: Query<(Entity, &Depiction), With<Door>>,
    mut doors_mut: Query<&mut Door>,
) {
    for command in commands.iter() {
        if let Command::Close(target) = &command.command {
            let (client, tile) =
                value_or_continue!(players.iter().find(|(c, _)| c.id == command.from));

            let siblings = value_or_continue!(tiles.get(tile.get()).ok());

            let Some(target) = target else {
                outbox.send_text(client.id, "Close what?");

                continue;
            };

            let Some((entity, _)) = siblings
                .iter()
                .flat_map(|siblings| siblings.iter())
                .filter_map(|sibling| doors.get(*sibling).ok())
                .find(|(entity, depiction)| depiction.matches_query(entity, target)) else {
                    outbox.send_text(client.id, "You can't close that.");

                    continue;
            };

            let Ok(mut door) = doors_mut.get_mut(entity) else {
                outbox.send_text(client.id, "You can't close that.");

                continue;
            };

            door.is_open = false;

            outbox.send_text(client.id, "You close the door.");
        }
    }
}
