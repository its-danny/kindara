use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;

use crate::{
    combat::commands::{
        advance::handle_advance, attack::handle_attack, block::handle_block, dodge::handle_dodge,
        retreat::handle_retreat, use_skill::handle_use_skill,
    },
    interact::{
        commands::{
            examine::handle_examine, place::handle_place, quit::handle_quit, roll::handle_roll,
            take::handle_take,
        },
        components::InMenu,
    },
    items::commands::{drop::handle_drop, inventory::handle_inventory},
    menu::commands::menu::handle_menu,
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
        stand::handle_stand,
    },
    visual::paint,
    world::commands::time::handle_time,
};

use super::events::{Command, ParseError, ParsedCommand, ProxyCommand};

#[sysfail(log)]
pub fn parse_command(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    mut commands: EventWriter<ParsedCommand>,
    clients: Query<(&Client, Option<&InMenu>), With<Online>>,
) -> Result<(), anyhow::Error> {
    for (input, content) in inbox.iter().filter_map(|m| {
        if let Message::Text(content) = &m.content {
            Some((m, paint::strip_style(content)))
        } else {
            None
        }
    }) {
        let (client, in_menu) = clients
            .iter()
            .find(|(c, _)| c.id == input.from)
            .context("Client not found")?;

        let handlers: Vec<Box<dyn Fn(&str) -> Result<Command, ParseError>>> = if in_menu.is_some() {
            vec![
                Box::new(handle_quit),
                // Menu is last because it captures any text.
                Box::new(handle_menu),
            ]
        } else {
            vec![
                Box::new(handle_advance),
                Box::new(handle_announce),
                Box::new(handle_attack),
                Box::new(handle_block),
                Box::new(handle_chat),
                Box::new(handle_close),
                Box::new(handle_config),
                Box::new(handle_describe),
                Box::new(handle_dodge),
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
                Box::new(handle_quit),
                Box::new(handle_retreat),
                Box::new(handle_roll),
                Box::new(handle_say),
                Box::new(handle_scan),
                Box::new(handle_sit),
                Box::new(handle_stand),
                Box::new(handle_take),
                Box::new(handle_time),
                Box::new(handle_who),
                Box::new(handle_yell),
                // Attack is last because the commands are a catch-all and
                // defined via ron files.
                Box::new(handle_use_skill),
            ]
        };

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

    Ok(())
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
