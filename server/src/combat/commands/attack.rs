use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    combat::{
        components::{Approach, CombatState, Distance, Stats},
        events::{CombatEvent, CombatEventKind, CombatEventTrigger},
    },
    data::resources::{Masteries, Skills},
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    npc::components::Npc,
    player::components::{Character, Client, Online},
    spatial::components::Tile,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_attack(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(attack|atk|a)( (?P<target>.*))?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Attack(target))
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct NpcQuery {
    entity: Entity,
    depiction: &'static Depiction,
    interactions: Option<&'static Interactions>,
    stats: Option<&'static Stats>,
    with_npc: With<Npc>,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct PlayerQuery {
    entity: Entity,
    client: &'static Client,
    tile: &'static Parent,
    character: &'static Character,
    combat_state: Option<&'static CombatState>,
    with_online: With<Online>,
    without_npc: Without<Npc>,
}

#[derive(WorldQuery)]
pub struct TileQuery {
    entity: Entity,
    children: &'static Children,
    with_tile: With<Tile>,
}

#[sysfail(log)]
pub fn attack(
    masteries: Res<Masteries>,
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut events: EventWriter<CombatEvent>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<PlayerQuery>,
    npcs: Query<NpcQuery>,
    skills: Res<Skills>,
    tiles: Query<TileQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Attack(target) = &command.command {
            let player = players
                .iter_mut()
                .find(|p| p.client.id == command.from)
                .context("Player not found")?;

            if player.combat_state.is_some() {
                outbox.send_text(player.client.id, "You are already in combat.");

                continue;
            }

            if let Some(target) = target {
                let target = match get_target(target, &tiles, &player.tile.get(), &npcs) {
                    Ok(entity) => entity,
                    Err(err) => {
                        outbox.send_text(player.client.id, err.to_string());

                        continue;
                    }
                };

                let skill_id = masteries
                    .0
                    .get(&player.character.mastery)
                    .with_context(|| format!("Mastery not found: {}", player.character.mastery))?
                    .auto_attack
                    .clone();

                let skill = skills
                    .0
                    .get(&skill_id)
                    .with_context(|| format!("Auto attack skill not found: {}", skill_id))?;

                bevy.entity(player.entity).insert(CombatState {
                    target,
                    distance: Distance::Near,
                    approach: Approach::Front,
                });

                bevy.entity(target).insert(CombatState {
                    target: player.entity,
                    distance: Distance::Near,
                    approach: Approach::Front,
                });

                events.send(CombatEvent {
                    source: player.entity,
                    trigger: CombatEventTrigger::Skill(skill.clone()),
                    kind: CombatEventKind::Attack,
                });
            }
        }
    }

    Ok(())
}

#[derive(Error, Debug, PartialEq)]
enum TargetError {
    #[error("You don't see anything here.")]
    NoTile,
    #[error("You don't see a {0} here.")]
    NotFound(String),
    #[error("You can't attack the {0}.")]
    Invalid(String),
}

fn get_target(
    target: &str,
    tiles: &Query<TileQuery>,
    tile: &Entity,
    npcs: &Query<NpcQuery>,
) -> Result<Entity, TargetError> {
    let siblings = tiles.get(*tile).ok().ok_or(TargetError::NoTile)?;

    let npc = siblings
        .children
        .iter()
        .filter_map(|sibling| npcs.get(*sibling).ok())
        .find(|npc| npc.depiction.matches_query(&npc.entity, target))
        .ok_or_else(|| TargetError::NotFound(target.into()))?;

    if !npc
        .interactions
        .map_or(false, |i| i.0.contains(&Interaction::Attack))
    {
        return Err(TargetError::Invalid(target.into()));
    }

    if npc.stats.is_none() {
        debug!("Target has Attack interaction but no state: {:?}", target);

        return Err(TargetError::Invalid(target.into()));
    }

    Ok(npc.entity)
}
