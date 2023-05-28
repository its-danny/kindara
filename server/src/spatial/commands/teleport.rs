use bevy::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::{
        components::{Character, Client},
        permissions,
    },
    spatial::components::{Position, Zone},
};

static REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(teleport|tp) (?P<zone>here|(.+)) \(((?P<x>\d) (?P<y>\d) (?P<z>\d))\)$").unwrap()
});

pub fn parse_teleport(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if let Some(captures) = REGEX.captures(content) {
        let region = captures.name("zone").map(|m| m.as_str()).unwrap_or("here");
        let x = captures
            .name("x")
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .unwrap_or_default();
        let y = captures
            .name("y")
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .unwrap_or_default();
        let z = captures
            .name("z")
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .unwrap_or_default();

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Teleport((region.to_string(), (x, y, z))),
        });

        true
    } else {
        false
    }
}

pub fn teleport(
    mut commands: EventReader<ParsedCommand>,
    mut players: Query<(&Client, &mut Position, &Character)>,
) {
    for command in commands.iter() {
        if let Command::Teleport((zone, (x, y, z))) = &command.command {
            let Some((_, mut player_position, character)) = players.iter_mut().find(|(c, _, _)| c.id == command.from) else {
                return;
            };

            if !character.can(permissions::TELEPORT) {
                return;
            }

            info!(
                "Teleporting {} to ({}, {}, {}) in {}",
                character.name, x, y, z, zone
            );

            if zone != "here" {
                player_position.zone = match zone.as_str() {
                    "movement" => Zone::Movement,
                    "void" => Zone::Void,
                    _ => Zone::Void,
                }
            }

            player_position.coords = IVec3::new(*x, *y, *z);
        }
    }
}
