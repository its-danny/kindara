use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{
    items::commands::{
        drop::handle_drop, inventory::handle_inventory, place::handle_place, take::handle_take,
    },
    player::{
        commands::config::handle_config,
        components::{Client, Online},
    },
    social::commands::{
        chat::handle_chat, emote::handle_emote, say::handle_say, who::handle_who, yell::handle_yell,
    },
    spatial::commands::{
        enter::handle_enter, look::handle_look, map::handle_map, movement::handle_movement,
        scan::handle_scan, teleport::handle_teleport,
    },
};

use super::events::{Command, ParseError, ParsedCommand, ProxyCommand};

pub fn parse_command(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    mut commands: EventWriter<ParsedCommand>,
    players: Query<&Client, With<Online>>,
) {
    for (message, content) in inbox.iter().filter_map(|m| {
        if let Message::Text(content) = &m.content {
            Some((m, content))
        } else {
            None
        }
    }) {
        let Some(client) = players.iter().find(|c| c.id == message.from) else {
            debug!("Could not find authenticated client: {:?}", message.from);

            continue;
        };

        let handlers: Vec<Box<dyn Fn(&str) -> Result<Command, ParseError>>> = vec![
            Box::new(handle_chat),
            Box::new(handle_config),
            Box::new(handle_drop),
            Box::new(handle_emote),
            Box::new(handle_enter),
            Box::new(handle_inventory),
            Box::new(handle_look),
            Box::new(handle_map),
            Box::new(handle_movement),
            Box::new(handle_place),
            Box::new(handle_say),
            Box::new(handle_scan),
            Box::new(handle_take),
            Box::new(handle_teleport),
            Box::new(handle_who),
            Box::new(handle_yell),
        ];

        match handlers.iter().find_map(|handler| match handler(content) {
            Err(ParseError::WrongCommand) => None,
            Ok(command) => Some(Ok(command)),
            Err(err) => Some(Err(err)),
        }) {
            Some(Ok(command)) => commands.send(ParsedCommand {
                from: client.id,
                command,
            }),
            Some(Err(error)) => outbox.send_text(client.id, error.to_string()),
            None => outbox.send_text(client.id, ParseError::UnknownCommand.to_string()),
        }
    }
}

pub fn handle_proxy_command(
    mut proxy: EventReader<ProxyCommand>,
    mut commands: EventWriter<ParsedCommand>,
) {
    for proxied in proxy.iter() {
        info!("Sending proxied command: {:?}", proxied.0);

        commands.send(proxied.0.clone());
    }
}
