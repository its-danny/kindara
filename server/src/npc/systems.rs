use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use bevy_proto::prelude::*;
use rand::{thread_rng, Rng};

use crate::{
    combat::components::{Attributes, HasAttacked, HitError, InCombat, State},
    player::{
        components::{Client, Online},
        events::Prompt,
    },
    skills::resources::Skills,
    spatial::components::Tile,
    visual::components::Depiction,
};

use super::components::{EnemySpawner, Npc, SpawnTimer};

#[sysfail(log)]
pub fn on_enter_combat(
    mut bevy: Commands,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut State), With<Online>>,
    mut prompts: EventWriter<Prompt>,
    npcs: Query<(Entity, &Npc, &Depiction, &Attributes, &InCombat), Added<InCombat>>,
    skills: Res<Skills>,
) -> Result<(), anyhow::Error> {
    for (entity, npc, depiction, attributes, in_combat) in npcs.iter() {
        let (client, mut state) = players.get_mut(in_combat.target)?;

        let mut rng = thread_rng();
        let index = rng.gen_range(0..npc.skills.len());
        let skill = skills
            .0
            .get(&npc.skills[index])
            .context("Skill not found")?;

        match in_combat.attack(&mut bevy, entity, skill, attributes, &mut state) {
            Ok(_) => {
                outbox.send_text(
                    client.id,
                    format!(
                        "{} attacks you. Your health is now {}.",
                        depiction.name, state.health
                    ),
                );
            }
            Err(HitError::Missed) => {
                outbox.send_text(
                    client.id,
                    format!("{} attacks you but misses.", depiction.name),
                );
            }
        }

        prompts.send(Prompt::new(client.id));
    }

    Ok(())
}

#[sysfail(log)]
pub fn attack_when_able(
    mut bevy: Commands,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut State), With<Online>>,
    mut prompts: EventWriter<Prompt>,
    mut ready: RemovedComponents<HasAttacked>,
    npcs: Query<(Entity, &Npc, &Depiction, &Attributes, &InCombat)>,
    skills: Res<Skills>,
) -> Result<(), anyhow::Error> {
    for entity in ready.iter() {
        let (entity, npc, depiction, attributes, in_combat) = npcs.get(entity)?;
        let (client, mut state) = players.get_mut(in_combat.target)?;

        let mut rng = thread_rng();
        let index = rng.gen_range(0..npc.skills.len());
        let skill = skills
            .0
            .get(&npc.skills[index])
            .context("Skill not found")?;

        match in_combat.attack(&mut bevy, entity, skill, attributes, &mut state) {
            Ok(_) => {
                outbox.send_text(
                    client.id,
                    format!(
                        "{} attacks you. Your health is now {}.",
                        depiction.name, state.health
                    ),
                );
            }
            Err(HitError::Missed) => {
                outbox.send_text(
                    client.id,
                    format!("{} attacks you but misses.", depiction.name),
                );
            }
        }

        prompts.send(Prompt::new(client.id));
    }

    Ok(())
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
