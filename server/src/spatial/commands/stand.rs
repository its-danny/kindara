use std::sync::OnceLock;

use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Client, Online},
    spatial::components::{Action, Seated},
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_stand(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^stand$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Stand),
    }
}

#[sysfail(log)]
pub fn stand(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(Entity, &Client, Option<&Seated>), With<Online>>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Stand = &command.command {
            let (player, client, seated) = players
                .iter()
                .find(|(_, c, _)| c.id == command.from)
                .context("Player not found")?;

            if seated.is_none() {
                outbox.send_text(client.id, "You are already standing.");

                continue;
            }

            bevy.entity(player).remove::<Seated>();
            bevy.entity(player).remove::<Action>();

            outbox.send_text(client.id, "You stand up.");
        }
    }

    Ok(())
}
