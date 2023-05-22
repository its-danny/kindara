use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::player::components::{Character, Client};

use super::components::{Impassable, Position, Tile};

pub(super) fn map(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position), With<Character>>,
    tiles: Query<(&Position, &Tile, Option<&Impassable>)>,
) {
    let regex = Regex::new("^(map|m)$").unwrap();

    for message in inbox
        .iter()
        .filter(|message| matches!(&message.content, Message::Text(text) if regex.is_match(text)))
    {
        let Some((_, position)) = players.iter().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        let width = 80;
        let height = 16;

        let mut map = vec![vec![' '; width]; height];

        let start_x = position.0.x - (width as i32 / 2);
        let end_x = position.0.x + (width as i32 / 2);
        let start_y = position.0.y - (height as i32 / 2);
        let end_y = position.0.y + (height as i32 / 2);

        for x in start_x..=end_x {
            for y in start_y..=end_y {
                if x == position.0.x && y == position.0.y {
                    map[(y - start_y) as usize][(x - start_x) as usize] = '@';
                } else if let Some((_, _, impassable)) = tiles
                    .iter()
                    .find(|(tile_position, _, _)| tile_position.0 == IVec3::new(x, y, position.0.z))
                {
                    map[(y - start_y) as usize][(x - start_x) as usize] =
                        if impassable.is_some() { '#' } else { '.' };
                }
            }
        }

        let display = map
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n");

        outbox.send_text(message.from, display);
    }
}

pub(super) fn movement(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Position), With<Character>>,
    tiles: Query<(&Position, &Tile, Option<&Impassable>), Without<Character>>,
) {
    let regex = Regex::new(
        "^(north|n|northeast|ne|east|e|southeast|se|south|s|southwest|sw|west|w|northwest|nw|up|u|down|d)$",
    )
    .unwrap();

    for message in inbox
        .iter()
        .filter(|message| matches!(&message.content, Message::Text(text) if regex.is_match(text)))
    {
        let Some((client, mut position)) = players.iter_mut().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        if let Message::Text(direction) = &message.content {
            let wanted_tile = match direction.as_str() {
                "north" | "n" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(0, -1, 0)),
                "northeast" | "ne" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(1, -1, 0)),
                "east" | "e" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(1, 0, 0)),
                "southeast" | "se" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(1, 1, 0)),
                "south" | "s" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(0, 1, 0)),
                "southwest" | "sw" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(-1, 1, 0)),
                "west" | "w" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(-1, 0, 0)),
                "northwest" | "nw" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(-1, -1, 0)),
                "up" | "u" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(0, 0, 1)),
                "down" | "d" => tiles
                    .iter()
                    .find(|(p, _, _)| p.0 == position.0 + IVec3::new(0, 0, -1)),
                _ => None,
            };

            if let Some((tile_position, _, impassable)) = wanted_tile {
                if impassable.is_none() {
                    position.0 = tile_position.0;
                } else {
                    outbox.send_text(client.0, "Something bars thy way.");
                }
            }
        }
    }
}
