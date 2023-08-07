use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use caith::Roller;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Character, Client, Online},
    spatial::components::Tile,
    value_or_continue,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_roll(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(roll( |$)|' ?)(?P<roll>.*)?$").unwrap());

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

pub fn roll(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Roll(roll) = &command.command {
            let (_, character, tile) =
                value_or_continue!(players.iter().find(|(c, _, _)| c.id == command.from));
            let siblings = value_or_continue!(tiles.get(tile.get()).ok());

            let roller = Roller::new(roll);

            if let Ok(roller) = roller {
                let result = roller.roll();

                if let Ok(result) = result {
                    for (client, _, _) in siblings.iter().filter_map(|c| players.get(*c).ok()) {
                        outbox.send_text(
                            client.id,
                            format!(
                                "{} rolled {} with a result of {}.",
                                character.name, roll, result
                            ),
                        );
                    }
                } else {
                    outbox.send_text(command.from, "Invalid roll.");
                }
            } else {
                outbox.send_text(command.from, "Invalid roll.");
            }
        }
    }
}
