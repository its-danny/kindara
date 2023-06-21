use bevy::prelude::*;
use bevy_proto::prelude::*;
use rand::{thread_rng, Rng};

use crate::{spatial::components::Tile, value_or_continue};

use super::components::{EnemySpawner, SpawnTimer};

pub fn handle_enemy_spawner(
    mut bevy: Commands,
    mut proto: ProtoCommands,
    mut spawners: Query<(Entity, &Parent, &mut EnemySpawner, Option<&mut SpawnTimer>)>,
    prototypes: Prototypes,
    tiles: Query<Entity, With<Tile>>,
    time: Res<Time>,
) {
    for (entity, tile, mut spawner, timer) in spawners.iter_mut() {
        if !prototypes.is_ready(&spawner.enemies.0) {
            continue;
        }

        let tile = value_or_continue!(tiles.get(tile.get()).ok());

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
}
