use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::Client,
    spatial::components::{Position, Tile},
    visual::components::Sprite,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn parse_map(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(map|m)$").unwrap());

    if regex.is_match(content) {
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
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Parent)>,
    tiles: Query<(&Position, &Sprite), With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Map = &command.command {
            let Some((client, parent)) = players.iter().find(|(c, _)| c.id == command.from) else {
                debug!("Could not find player for client: {:?}", command.from);

                continue;
            };

            let Ok((player_position, _)) = tiles.get(parent.get()) else {
                debug!("Could not get parent: {:?}", parent.get());

                continue;
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
                    } else if let Some((_, sprite)) = tiles.iter().find(|(p, _)| {
                        p.zone == player_position.zone
                            && p.coords.x == x
                            && p.coords.y == y
                            && p.coords.z == player_position.coords.z
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

            outbox.send_text(client.id, format!("{}\n{display}", player_position.zone));
        }
    }
}
