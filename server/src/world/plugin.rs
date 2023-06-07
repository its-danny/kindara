use bevy::prelude::*;
use bevy_proto::prelude::*;

use super::systems::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(spawn_void.run_if(prototype_ready("world.void").and_then(run_once())))
            .add_system(
                spawn_testing_movement
                    .run_if(prototype_ready("world.testing.movement").and_then(run_once())),
            );
    }
}
