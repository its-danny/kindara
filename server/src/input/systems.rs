use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{
    player::{
        commands::config::handle_config,
        components::{Character, Client},
    },
    social::commands::{say::handle_say, who::handle_who},
    spatial::commands::{
        enter::handle_enter, look::handle_look, map::handle_map, movement::handle_movement,
        teleport::handle_teleport,
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

        if handle_config(client, content, &mut commands)
            || handle_enter(client, content, &mut commands)
            || handle_look(client, content, &mut commands)
            || handle_map(client, content, &mut commands)
            || handle_movement(client, content, &mut commands)
            || handle_say(client, content, &mut commands)
            || handle_teleport(client, content, &mut commands)
            || handle_who(client, content, &mut commands)
        {
            continue;
        }

        outbox.send_text(client.id, "Unknown command.");
    }
}
