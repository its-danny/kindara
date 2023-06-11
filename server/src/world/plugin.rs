use bevy::prelude::*;
use bevy_proto::prelude::*;

use super::systems::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            spawn_trinus_castra
                .run_if(prototype_ready("world.trinus.trinus-castra").and_then(run_once())),
            spawn_the_roaring_lion
                .run_if(prototype_ready("world.trinus.the-roaring-lion").and_then(run_once())),
        ));
    }
}
