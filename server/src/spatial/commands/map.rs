use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Client, Online},
    spatial::components::{Position, Tile, Zone},
    value_or_continue,
    visual::components::Sprite,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_map(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(map|m)$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Map),
    }
}

pub fn map(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Parent), With<Online>>,
    tiles: Query<(&Position, &Sprite, &Parent), With<Tile>>,
    zones: Query<(&Zone, &Children)>,
) {
    for command in commands.iter() {
        if let Command::Map = &command.command {
            let (client, tile) =
                value_or_continue!(players.iter().find(|(c, _)| c.id == command.from));
            let (position, _, zone) = value_or_continue!(tiles.get(tile.get()).ok());
            let (zone, zone_tiles) = value_or_continue!(zones.get(zone.get()).ok());

            let height = 24;
            let width = if client.width % 2 == 1 {
                client.width - 1
            } else {
                client.width
            } as usize;

            let mut map = vec![vec![' '; width]; height];

            let start_x = position.0.x - (width as i32 / 2);
            let end_x = position.0.x + (width as i32 / 2);
            let start_y = position.0.y - (height as i32 / 2);
            let end_y = position.0.y + (height as i32 / 2);

            for x in start_x..=end_x {
                for y in start_y..=end_y {
                    if x == position.0.x && y == position.0.y {
                        map[(y - start_y) as usize][(x - start_x) as usize] = '@';
                    } else if let Some(sprite) = zone_tiles.iter().find_map(|child| {
                        tiles
                            .get(*child)
                            .ok()
                            .filter(|(p, _, _)| p.0 == IVec3::new(x, y, position.0.z))
                            .map(|(_, s, _)| s)
                    }) {
                        map[(y - start_y) as usize][(x - start_x) as usize] =
                            sprite.character.chars().next().unwrap_or(' ');
                    }
                }
            }

            let display = map
                .iter()
                .map(|row| row.iter().collect::<String>())
                .collect::<Vec<_>>()
                .join("\n");

            outbox.send_text(client.id, format!("{}\n{display}", zone.name));
        }
    }
}
