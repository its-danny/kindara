use std::sync::OnceLock;

use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::InMenu,
    player::components::{Client, Online},
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_quit(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^quit$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Quit),
    }
}

#[sysfail(log)]
pub fn quit(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    server: Res<Server>,
    players: Query<(Entity, &Client, Option<&InMenu>), With<Online>>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Quit = &command.command {
            let (player, client, in_menu) = players
                .iter()
                .find(|(_, c, _)| c.id == command.from)
                .context("Player not found")?;

            if in_menu.is_some() {
                bevy.entity(player).remove::<InMenu>();
            } else {
                outbox.send_text(client.id, "Farewell.");
                server.disconnect(&client.id);
            }
        }
    }

    Ok(())
}
