use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use bevy_proto::prelude::*;
use rand::{thread_rng, Rng};
use thiserror::Error;

use crate::{
    combat::components::{HasAttacked, HitError, InCombat, Stats},
    player::{
        components::{Client, Online},
        events::Prompt,
    },
    skills::{components::Cooldowns, resources::Skills},
    spatial::components::Tile,
    visual::components::Depiction,
};

use super::components::{EnemySpawner, Npc, SpawnTimer};

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct PlayerQuery {
    pub client: &'static Client,
    pub stats: &'static mut Stats,
    with_online: With<Online>,
    without_npc: Without<Npc>,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct NpcQuery {
    pub entity: Entity,
    pub npc: &'static Npc,
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
    ready: Query<Entity, (With<Npc>, With<InCombat>, Without<HasAttacked>)>,
    mut npcs: Query<NpcQuery>,
) -> Result<(), anyhow::Error> {
    for entity in ready.iter() {
        let mut npc = npcs.get_mut(entity)?;
        let mut player = players.get_mut(npc.in_combat.target)?;

        if let Ok(Some(message)) = perform_attack(
            &mut bevy,
            &skills,
            entity,
            &mut player.stats,
            npc.npc,
            &mut npc.cooldowns,
            npc.stats,
            npc.in_combat,
            npc.depiction,
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
    npc: &Npc,
    npc_cooldowns: &mut Cooldowns,
    npc_stats: &Stats,
    npc_in_combat: &InCombat,
    npc_depiction: &Depiction,
) -> Result<Option<String>, anyhow::Error> {
    if npc.skills.is_empty() {
        Err(AttackError::NoSkills)?
    }

    if npc_cooldowns.0.contains_key(&npc.skills[0]) {
        return Ok(None);
    }

    let mut rng = thread_rng();
    let index = rng.gen_range(0..npc.skills.len());
    let skill = skills
        .0
        .get(&npc.skills[index])
        .context("Skill not found")?;

    npc_cooldowns.0.insert(
        skill.id.clone(),
        Timer::from_seconds(skill.cooldown as f32, TimerMode::Once),
    );

    match npc_in_combat.attack(bevy, entity, skill, npc_stats, player_stats) {
        Ok(_) => Ok(Some(format!("{} attacks you.", npc_depiction.name,))),
        Err(HitError::Dodged) => Ok(Some("You dodge their attack.".into())),
        Err(HitError::Blocked) => Ok(Some("You block their attack.".into())),
    }
}

#[sysfail(log)]
pub fn handle_enemy_spawner(
    mut bevy: Commands,
    mut proto: ProtoCommands,
    mut spawners: Query<(Entity, &Parent, &mut EnemySpawner, Option<&mut SpawnTimer>)>,
    prototypes: Prototypes,
    tiles: Query<Entity, With<Tile>>,
    time: Res<Time>,
) -> Result<(), anyhow::Error> {
    for (entity, tile, mut spawner, timer) in spawners.iter_mut() {
        if !prototypes.is_ready(&spawner.enemies.0) {
            continue;
        }

        let tile = tiles.get(tile.get())?;

        if let Some(mut timer) = timer {
            if timer.0.tick(time.delta()).just_finished() {
                let mut rng = thread_rng();
                let range = spawner.enemies.1..=spawner.enemies.2;
                let amount = rng.gen_range(range);

                for _ in 0..amount {
                    let enemy = proto.spawn(&spawner.enemies.0);
                    bevy.entity(enemy.id()).set_parent(tile);

                    spawner.spawned.push(enemy.id());
                }

                bevy.entity(entity).remove::<SpawnTimer>();
            }
        } else if spawner.spawned.is_empty() {
            bevy.entity(entity).insert(SpawnTimer(Timer::from_seconds(
                spawner.delay,
                TimerMode::Once,
            )));
        }
    }

    Ok(())
}
