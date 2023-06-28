use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Client, Online},
    value_or_continue,
    world::resources::WorldTime,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_time(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^time$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Time),
    }
}

pub fn time(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<&Client, With<Online>>,
    world_time: Res<WorldTime>,
) {
    for command in commands.iter() {
        if let Command::Time = &command.command {
            let client = value_or_continue!(players.iter().find(|c| c.id == command.from));

            outbox.send_text(client.id, world_time.to_string());
        }
    }
}
