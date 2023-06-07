use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{
    player::{
        commands::config::parse_config,
        components::{Character, Client},
    },
    social::commands::{say::parse_say, who::parse_who},
    spatial::commands::{
        enter::parse_enter, look::parse_look, map::parse_map, movement::parse_movement,
        teleport::parse_teleport,
    },
};

use super::events::ParsedCommand;

pub fn parse_command(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    mut commands: EventWriter<ParsedCommand>,
    players: Query<&Client, With<Character>>,
) {
    for (message, content) in inbox.iter().filter_map(|m| {
        if let Message::Text(content) = &m.content {
            Some((m, content))
        } else {
            None
        }
    }) {
        let Some(client) = players.iter().find(|c| c.id == message.from) else {
            debug!("Could not find player for client: {:?}", message.from);

            continue;
        };

        if parse_config(client, content, &mut commands)
            || parse_enter(client, content, &mut commands)
            || parse_look(client, content, &mut commands)
            || parse_map(client, content, &mut commands)
            || parse_movement(client, content, &mut commands)
            || parse_say(client, content, &mut commands)
            || parse_teleport(client, content, &mut commands)
            || parse_who(client, content, &mut commands)
        {
            continue;
        }

        outbox.send_text(client.id, "Unknown command.");
    }
}
