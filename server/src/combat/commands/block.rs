use std::sync::OnceLock;

use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    combat::components::{BlockCooldown, ManualBlock, Stats},
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Client, Online},
    values::MANUAL_BLOCK_TIMER,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_block(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^block$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Block),
    }
}

#[sysfail(log)]
pub fn block(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    players: Query<(Entity, &Client, &Stats, Option<&BlockCooldown>), With<Online>>,
    mut outbox: EventWriter<Outbox>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Block = &command.command {
            let (entity, _, stats, block_cooldown) = players
                .iter()
                .find(|(_, c, _, _)| c.id == command.from)
                .context("Player not found")?;

            if block_cooldown.is_some() {
                outbox.send_text(command.from, "You are not ready to block again.");

                continue;
            }

            bevy.entity(entity).insert(ManualBlock(Timer::from_seconds(
                MANUAL_BLOCK_TIMER,
                TimerMode::Once,
            )));

            bevy.entity(entity)
                .insert(BlockCooldown(Timer::from_seconds(
                    stats.block_cooldown(),
                    TimerMode::Once,
                )));

            outbox.send_text(command.from, "You prepare to block.");
        }
    }

    Ok(())
}
