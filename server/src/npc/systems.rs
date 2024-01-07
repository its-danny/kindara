use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_proto::prelude::*;
use rand::{thread_rng, Rng};

use crate::spatial::components::Tile;

use super::components::{HostileSpawnTimer, HostileSpawner};

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

        if !spawner.initial_spawn {
            spawn_hostiles(&mut bevy, &mut proto, &mut spawner, entity, tile);

            spawner.initial_spawn = true;
        }

        if let Some(mut timer) = timer {
            if timer.0.tick(time.delta()).just_finished() {
                spawn_hostiles(&mut bevy, &mut proto, &mut spawner, entity, tile);
            }
        } else if spawner.spawned.is_empty() {
            bevy.entity(entity)
                .insert(HostileSpawnTimer(Timer::from_seconds(
                    spawner.delay,
                    TimerMode::Once,
                )));
        }
    }

    fn spawn_hostiles(
        bevy: &mut Commands,
        proto: &mut ProtoCommands,
        spawner: &mut HostileSpawner,
        entity: Entity,
        tile: Entity,
    ) {
        if spawner.spawned.len() >= spawner.hostiles.2.into() {
            return;
        }

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

    Ok(())
}
