use bevy::prelude::*;

use super::{
    bundles::TileBundle,
    commands::{enter::*, look::*, map::*, movement::*, scan::*, teleport::*},
    components::*,
};

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Vec<String>>()
            .register_type::<Position>()
            .register_type::<Tile>()
            .register_type::<Spawn>()
            .register_type::<TileBundle>()
            .register_type::<Transition>()
            .register_type::<Zone>()
            .add_systems((look, scan, map, movement, enter, teleport));
    }
}
