use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    player::{
        components::{Character, Client},
        permissions,
    },
    visual::components::Sprite,
};

use super::{
    components::{Impassable, Position, Tile, Zone},
    utils::view_for_tile,
};

pub(super) fn look(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position), With<Character>>,
    tiles: Query<(&Position, &Tile, &Sprite)>,
) {
    let regex = Regex::new(r"^(look|l)$").unwrap();

    for (message, _) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) if regex.is_match(text) => Some((message, text)),
        _ => None,
    }) {
        let Some((client, position)) = players.iter().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        let Some((_, tile, sprite)) = tiles
            .iter()
            .find(|(p, _, _)| p.zone == position.zone && p.coords == position.coords) else {
                return;
            };

        outbox.send_text(client.0, view_for_tile(tile, sprite));
    }
}

pub(super) fn map(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position), With<Character>>,
    tiles: Query<(&Position, &Tile, &Sprite)>,
) {
    let regex = Regex::new(r"^(map|m)$").unwrap();

    for (message, _) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) if regex.is_match(text) => Some((message, text)),
        _ => None,
    }) {
        let Some((client, position)) = players.iter().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        let width = 64;
        let height = 16;

        let mut map = vec![vec![" "; width]; height];

        let start_x = position.coords.x - (width as i32 / 2);
        let end_x = position.coords.x + (width as i32 / 2);
        let start_y = position.coords.y - (height as i32 / 2);
        let end_y = position.coords.y + (height as i32 / 2);

        for x in start_x..=end_x {
            for y in start_y..=end_y {
                if x == position.coords.x && y == position.coords.y {
                    map[(y - start_y) as usize][(x - start_x) as usize] = "@";
                } else if let Some((_, _, sprite)) = tiles.iter().find(|(tile_position, _, _)| {
                    tile_position.zone == position.zone
                        && tile_position.coords == IVec3::new(x, y, position.coords.z)
                }) {
                    map[(y - start_y) as usize][(x - start_x) as usize] = &sprite.character;
                }
            }
        }

        let display = map
            .iter()
            .map(|row| row.join(""))
            .collect::<Vec<_>>()
            .join("\n");

        outbox.send_text(client.0, format!("{}\n{}", position.zone, display));
    }
}

pub(super) fn movement(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Position), With<Character>>,
    tiles: Query<(&Position, &Tile, &Sprite, Option<&Impassable>), Without<Character>>,
) {
    let regex = Regex::new(
        r"^(north|n|northeast|ne|east|e|southeast|se|south|s|southwest|sw|west|w|northwest|nw|up|u|down|d)$",
    )
    .unwrap();

    for (message, direction) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) if regex.is_match(text) => Some((message, text)),
        _ => None,
    }) {
        let Some((client, mut position)) = players.iter_mut().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        let tiles = tiles
            .iter()
            .filter(|(p, _, _, _)| p.zone == position.zone)
            .collect::<Vec<_>>();

        let wanted_tile = match direction.as_str() {
            "north" | "n" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(0, -1, 0)),
            "northeast" | "ne" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(1, -1, 0)),
            "east" | "e" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(1, 0, 0)),
            "southeast" | "se" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(1, 1, 0)),
            "south" | "s" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(0, 1, 0)),
            "southwest" | "sw" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(-1, 1, 0)),
            "west" | "w" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(-1, 0, 0)),
            "northwest" | "nw" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(-1, -1, 0)),
            "up" | "u" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(0, 0, 1)),
            "down" | "d" => tiles
                .iter()
                .find(|(p, _, _, _)| p.coords == position.coords + IVec3::new(0, 0, -1)),
            _ => None,
        };

        if let Some((tile_position, tile, sprite, impassable)) = wanted_tile {
            if impassable.is_none() {
                position.coords = tile_position.coords;

                outbox.send_text(client.0, view_for_tile(tile, sprite))
            } else {
                outbox.send_text(client.0, "Something blocks your path.");
            }
        }
    }
}

// USAGE: (teleport|tp) (here|<zone>) (<x> <y> <z>)
pub(super) fn teleport(
    mut inbox: EventReader<Inbox>,
    mut players: Query<(&Client, &mut Position, &Character)>,
) {
    let regex = Regex::new(r"^(teleport|tp) (here|(.+)) \(((\d) (\d) (\d))\)$").unwrap();

    for (message, captures) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) => regex.captures(text).map(|caps| (message, caps)),
        _ => None,
    }) {
        let Some((_, mut position, character)) = players.iter_mut().find(|(c, _, _)| c.0 == message.from) else {
            return;
        };

        if !character.can(permissions::TELEPORT) {
            return;
        }

        let region = captures.get(2).map(|m| m.as_str()).unwrap_or("here");
        let x = captures
            .get(5)
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .unwrap_or_default();
        let y = captures
            .get(6)
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .unwrap_or_default();
        let z = captures
            .get(7)
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .unwrap_or_default();

        info!(
            "Teleporting {} to ({}, {}, {}) in {}",
            character.name, x, y, z, region
        );

        if region != "here" {
            position.zone = match region {
                "movement" => Zone::Movement,
                "void" => Zone::Void,
                _ => Zone::Void,
            }
        }

        position.coords = IVec3::new(x, y, z);
    }
}
