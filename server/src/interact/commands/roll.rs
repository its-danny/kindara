use std::sync::OnceLock;

use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use caith::Roller;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Character, Client, Online},
    spatial::components::Tile,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_roll(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(roll( |$)|# ?)(?P<roll>.*)?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let roll = captures
                .name("roll")
                .map(|m| m.as_str().trim())
                .filter(|m| !m.is_empty())
                .ok_or(ParseError::InvalidArguments("Roll what?".into()))?;

            Ok(Command::Roll(roll.into()))
        }
    }
}

#[sysfail(log)]
pub fn roll(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Roll(roll) = &command.command {
            let (client, character, tile) = players
                .iter()
                .find(|(c, _, _)| c.id == command.from)
                .context("Player not found")?;

            let siblings = tiles.get(tile.get())?;

            let roller = match Roller::new(roll) {
                Ok(roller) => roller,
                Err(_) => {
                    outbox.send_text(client.id, "Invalid roll syntax.");
                    continue;
                }
            };

            match roller.roll() {
                Ok(result) => {
                    siblings
                        .iter()
                        .filter_map(|c| players.get(*c).ok())
                        .for_each(|(client, _, _)| {
                            outbox.send_text(
                                client.id,
                                format!(
                                    "{} rolled {} with a result of {}.",
                                    character.name, roll, result
                                ),
                            );
                        });
                }
                Err(_) => outbox.send_text(client.id, "Roll failed."),
            }
        }
    }

    Ok(())
}
