use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    player::{
        components::{Character, Client},
        permissions,
    },
    spatial::components::{Position, Zone},
};

// USAGE: (teleport|tp) (here|<zone>) (<x> <y> <z>)
pub fn teleport(
    mut inbox: EventReader<Inbox>,
    mut players: Query<(&Client, &mut Position, &Character)>,
) {
    let regex =
        Regex::new(r"^(teleport|tp) (?P<zone>here|(.+)) \(((?P<x>\d) (?P<y>\d) (?P<z>\d))\)$")
            .unwrap();

    for (message, captures) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) => regex.captures(text).map(|caps| (message, caps)),
        _ => None,
    }) {
        let Some((_, mut player_position, character)) = players.iter_mut().find(|(c, _, _)| c.0 == message.from) else {
            return;
        };

        if !character.can(permissions::TELEPORT) {
            return;
        }

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

        info!(
            "Teleporting {} to ({}, {}, {}) in {}",
            character.name, x, y, z, region
        );

        if region != "here" {
            player_position.zone = match region {
                "movement" => Zone::Movement,
                "void" => Zone::Void,
                _ => Zone::Void,
            }
        }

        player_position.coords = IVec3::new(x, y, z);
    }
}
