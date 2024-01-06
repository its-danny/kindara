use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use bevy_proto::prelude::*;
use rand::{thread_rng, Rng};
use thiserror::Error;

use crate::{
    combat::components::{Cooldowns, HasAttacked, HitError, InCombat, Stats},
    player::{
        components::{Client, Online},
        events::Prompt,
    },
    skills::resources::Skills,
    spatial::components::Tile,
    visual::components::Depiction,
};

use super::components::{Hostile, HostileSpawnTimer, HostileSpawner};

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct PlayerQuery {
    pub client: &'static Client,
    pub stats: &'static mut Stats,
    with_online: With<Online>,
    without_hostile: Without<Hostile>,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct HostileQuery {
    pub entity: Entity,
    pub hostile: &'static Hostile,
    pub depiction: &'static Depiction,
    pub stats: &'static Stats,
    pub cooldowns: &'static mut Cooldowns,
    pub in_combat: &'static InCombat,
}

#[sysfail(log)]
pub fn attack_when_able(
    mut bevy: Commands,
    mut outbox: EventWriter<Outbox>,
    skills: Res<Skills>,
    mut players: Query<PlayerQuery>,
    mut prompts: EventWriter<Prompt>,
    ready: Query<Entity, (With<Hostile>, With<InCombat>, Without<HasAttacked>)>,
    mut hostiles: Query<HostileQuery>,
) -> Result<(), anyhow::Error> {
    for entity in ready.iter() {
        let mut hostile = hostiles.get_mut(entity)?;
        let mut player = players.get_mut(hostile.in_combat.target)?;

        if let Ok(Some(message)) = perform_attack(
            &mut bevy,
            &skills,
            entity,
            &mut player.stats,
            hostile.hostile,
            &mut hostile.cooldowns,
            hostile.stats,
            hostile.in_combat,
            hostile.depiction,
        ) {
            outbox.send_text(player.client.id, message);
            prompts.send(Prompt::new(player.client.id));
        }
    }

    Ok(())
}

#[derive(Error, Debug, PartialEq)]
enum AttackError {
    #[error("NPC entered combat with no skills")]
    NoSkills,
}

fn perform_attack(
    bevy: &mut Commands,
    skills: &Res<Skills>,
    entity: Entity,
    player_stats: &mut Stats,
    hostile: &Hostile,
    hostile_cooldowns: &mut Cooldowns,
    hostile_stats: &Stats,
    hostile_in_combat: &InCombat,
    hostile_depiction: &Depiction,
) -> Result<Option<String>, anyhow::Error> {
    if hostile.skills.is_empty() {
        Err(AttackError::NoSkills)?
    }

    if hostile_cooldowns.0.contains_key(&hostile.skills[0]) {
        return Ok(None);
    }

    let mut rng = thread_rng();
    let index = rng.gen_range(0..hostile.skills.len());
    let skill = skills
        .0
        .get(&hostile.skills[index])
        .context("Skill not found")?;

    hostile_cooldowns.0.insert(
        skill.id.clone(),
        Timer::from_seconds(skill.cooldown as f32, TimerMode::Once),
    );

    match hostile_in_combat.attack(bevy, entity, skill, hostile_stats, player_stats) {
        Ok(_) => Ok(Some(format!("{} attacks you.", hostile_depiction.name,))),
        Err(HitError::Dodged) => Ok(Some("You dodge their attack.".into())),
        Err(HitError::Blocked) => Ok(Some("You block their attack.".into())),
    }
}

#[sysfail(log)]
pub fn handle_hostile_spawner(
    mut bevy: Commands,
    mut proto: ProtoCommands,
    mut spawners: Query<(
        Entity,
        &Parent,
        &mut HostileSpawner,
        Option<&mut HostileSpawnTimer>,
    )>,
    prototypes: Prototypes,
    tiles: Query<Entity, With<Tile>>,
    time: Res<Time>,
) -> Result<(), anyhow::Error> {
    for (entity, tile, mut spawner, timer) in spawners.iter_mut() {
        if !prototypes.is_ready(&spawner.hostiles.0) {
            continue;
        }

        let tile = tiles.get(tile.get())?;

        if let Some(mut timer) = timer {
            if timer.0.tick(time.delta()).just_finished() {
                let mut rng = thread_rng();
                let range = spawner.hostiles.1..=spawner.hostiles.2;
                let amount = rng.gen_range(range);

                for _ in 0..amount {
                    let hostile = proto.spawn(&spawner.hostiles.0);
                    bevy.entity(hostile.id()).set_parent(tile);

                    spawner.spawned.push(hostile.id());
                }

                bevy.entity(entity).remove::<HostileSpawnTimer>();
            }
        } else if spawner.spawned.is_empty() {
            bevy.entity(entity)
                .insert(HostileSpawnTimer(Timer::from_seconds(
                    spawner.delay,
                    TimerMode::Once,
                )));
        }
    }

    Ok(())
}
