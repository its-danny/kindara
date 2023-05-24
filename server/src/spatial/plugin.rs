use bevy::prelude::*;

use super::{
    bundles::{TileBundle, TransitionBundle},
    commands::{enter::*, look::*, map::*, movement::*, teleport::*},
    components::{Position, Tile, Transition, Zone},
};

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Vec<String>>()
            .register_type::<Position>()
            .register_type::<Tile>()
            .register_type::<TileBundle>()
            .register_type::<Transition>()
            .register_type::<TransitionBundle>()
            .register_type::<Zone>()
            .add_system(look)
            .add_system(map)
            .add_system(movement)
            .add_system(enter)
            .add_system(teleport);
    }
}
