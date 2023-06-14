use std::time::Duration;

use bevy::prelude::*;
use bevy_proto::prelude::*;

use crate::Set;

use super::{
    resources::{SaveTimer, WorldState},
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
            .add_systems((
                save_world_state.in_base_set(Set::WorldSave),
                handle_save_world_state_task,
                handle_load_world_state_task,
            ))
            .add_startup_system(load_world_state);

        app.add_systems((
            spawn_trinus_castra
                .run_if(prototype_ready("world.trinus.trinus-castra").and_then(run_once())),
            spawn_the_roaring_lion
                .run_if(prototype_ready("world.trinus.the-roaring-lion").and_then(run_once())),
        ));
    }
}
