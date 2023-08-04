use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{
    combat::commands::attack::handle_attack,
    interact::commands::{examine::handle_examine, place::handle_place, take::handle_take},
    items::commands::{drop::handle_drop, inventory::handle_inventory},
    player::{
        commands::{config::handle_config, describe::handle_describe},
        components::{Client, Online},
    },
    social::commands::{
        announce::handle_announce, chat::handle_chat, emote::handle_emote, say::handle_say,
        who::handle_who, yell::handle_yell,
    },
    spatial::commands::{
        close::handle_close, enter::handle_enter, look::handle_look, map::handle_map,
        movement::handle_movement, open::handle_open, scan::handle_scan, sit::handle_sit,
        stand::handle_stand, teleport::handle_teleport,
    },
    value_or_continue,
    visual::paint,
    world::commands::time::handle_time,
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
            Some((m, paint::strip_style(content)))
        } else {
            None
        }
    }) {
        let client = value_or_continue!(players.iter().find(|c| c.id == message.from));

        let handlers: Vec<Box<dyn Fn(&str) -> Result<Command, ParseError>>> = vec![
            Box::new(handle_announce),
            Box::new(handle_chat),
            Box::new(handle_close),
            Box::new(handle_config),
            Box::new(handle_describe),
            Box::new(handle_drop),
            Box::new(handle_emote),
            Box::new(handle_enter),
            Box::new(handle_examine),
            Box::new(handle_inventory),
            Box::new(handle_look),
            Box::new(handle_map),
            Box::new(handle_movement),
            Box::new(handle_open),
            Box::new(handle_place),
            Box::new(handle_say),
            Box::new(handle_scan),
            Box::new(handle_sit),
            Box::new(handle_stand),
            Box::new(handle_take),
            Box::new(handle_teleport),
            Box::new(handle_time),
            Box::new(handle_who),
            Box::new(handle_yell),
            // Attack is last because the commands are a catch-all and
            // defined via ron files.
            Box::new(handle_attack),
        ];

        match handlers.iter().find_map(|handler| match handler(&content) {
            Err(ParseError::WrongCommand) => None,
            Ok(command) => Some(Ok(command)),
            Err(err) => Some(Err(err)),
        }) {
            Some(Ok(command)) => {
                debug!("Parsed command: {:?}", command);

                commands.send(ParsedCommand {
                    from: client.id,
                    command,
                })
            }
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
