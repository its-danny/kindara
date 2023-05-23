use bevy::prelude::*;
use bevy_proto::prelude::*;

use super::{resources::TileMap, systems::*};

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(TileMap::default())
            .add_system(spawn_void.run_if(prototype_ready("world.void").and_then(run_once())))
            .add_system(
                spawn_testing_movement
                    .run_if(prototype_ready("world.testing.movement").and_then(run_once())),
            )
            .add_systems((on_tile_added, on_tile_removed));
    }
}
