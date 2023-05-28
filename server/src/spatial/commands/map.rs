use bevy::prelude::*;
use bevy_nest::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
    spatial::components::{Position, Tile},
    visual::components::Sprite,
    world::resources::TileMap,
};

static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(map|m)$").unwrap());

pub fn parse_map(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if REGEX.is_match(content) {
        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Map,
        });

        true
    } else {
        false
    }
}

pub fn map(
    tile_map: Res<TileMap>,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position), With<Character>>,
    tiles: Query<&Sprite, With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Map = &command.command {
            let Some((client, player_position)) = players.iter().find(|(c, _)| c.id == command.from) else {
                return;
            };

            let height = 24;
            let width = if client.width % 2 == 1 {
                client.width - 1
            } else {
                client.width
            } as usize;

            let mut map = vec![vec![' '; width]; height];

            let start_x = player_position.coords.x - (width as i32 / 2);
            let end_x = player_position.coords.x + (width as i32 / 2);
            let start_y = player_position.coords.y - (height as i32 / 2);
            let end_y = player_position.coords.y + (height as i32 / 2);

            for x in start_x..=end_x {
                for y in start_y..=end_y {
                    if x == player_position.coords.x && y == player_position.coords.y {
                        map[(y - start_y) as usize][(x - start_x) as usize] = '@';
                    } else if let Some(sprite) = tile_map
                        .get(
                            player_position.zone,
                            IVec3::new(x, y, player_position.coords.z),
                        )
                        .and_then(|e| tiles.get(*e).ok())
                    {
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

            outbox.send_text(client.id, format!("{}\n{display}", player_position.zone));
        }
    }
}
