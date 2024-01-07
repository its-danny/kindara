use std::sync::OnceLock;

use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    combat::components::{DodgeCooldown, ManualDodge, Stats},
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Client, Online},
    values::MANUAL_DODGE_TIMER,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_dodge(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^dodge$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Dodge),
    }
}

#[sysfail(log)]
pub fn dodge(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    players: Query<(Entity, &Client, &Stats, Option<&DodgeCooldown>), With<Online>>,
    mut outbox: EventWriter<Outbox>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Dodge = &command.command {
            let (entity, _, stats, dodge_cooldown) = players
                .iter()
                .find(|(_, c, _, _)| c.id == command.from)
                .context("Player not found")?;

            if dodge_cooldown.is_some() {
                outbox.send_text(command.from, "You are not ready to dodge again.");

                continue;
            }

            bevy.entity(entity).insert(ManualDodge(Timer::from_seconds(
                MANUAL_DODGE_TIMER,
                TimerMode::Once,
            )));

            bevy.entity(entity)
                .insert(DodgeCooldown(Timer::from_seconds(
                    stats.dodge_cooldown(),
                    TimerMode::Once,
                )));

            outbox.send_text(command.from, "You prepare to dodge.");
        }
    }

    Ok(())
}
