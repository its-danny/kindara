use std::sync::OnceLock;

use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    combat::components::{AttackTimer, CombatState, Distance, Stats},
    input::events::{Command, ParseError, ParsedCommand},
    npc::components::Hostile,
    player::components::{Client, Online},
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_advance(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^advance|adv$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Advance),
    }
}

#[sysfail(log)]
pub fn advance(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut players: Query<(Entity, &Client, &Stats, Option<&mut CombatState>), With<Online>>,
    mut hostiles: Query<&mut CombatState, (With<Hostile>, Without<Online>)>,
    mut outbox: EventWriter<Outbox>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Advance = &command.command {
            let (entity, _, stats, combat_state) = players
                .iter_mut()
                .find(|(_, c, _, _)| c.id == command.from)
                .context("Player not found")?;

            if let Some(mut combat_state) = combat_state {
                combat_state.distance = Distance::Near;

                let mut hostile = hostiles.get_mut(combat_state.target)?;
                hostile.distance = Distance::Near;

                bevy.entity(entity).insert(AttackTimer(Timer::from_seconds(
                    stats.attack_speed(),
                    TimerMode::Once,
                )));

                outbox.send_text(command.from, "You advance on your target.");
            } else {
                outbox.send_text(command.from, "You are not in combat.");
            }
        }
    }

    Ok(())
}
