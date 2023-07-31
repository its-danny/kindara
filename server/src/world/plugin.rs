use std::time::Duration;

use bevy::prelude::*;
use bevy_proto::prelude::*;

use super::{
    commands::time::*,
    resources::{SaveTimer, WorldState, WorldTime},
    systems::*,
};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WorldState::default())
            .insert_resource(SaveTimer(Timer::new(
                Duration::from_secs(60),
                TimerMode::Repeating,
            )))
            .insert_resource(WorldTime::default())
            .add_systems(Last, save_world_state)
            .add_systems(
                Update,
                (handle_save_world_state_task, handle_load_world_state_task),
            )
            .add_systems(Startup, (load_world_state,))
            .add_systems(Update, (time, update_world_time));

        app.add_systems(
            Update,
            (
                spawn_trinus_castra
                    .run_if(prototype_ready("world.trinus.trinus-castra").and_then(run_once())),
                spawn_the_roaring_lion
                    .run_if(prototype_ready("world.trinus.the-roaring-lion").and_then(run_once())),
            ),
        );
    }
}
