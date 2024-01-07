use bevy::prelude::*;

use super::{
    bundles::{TileBundle, TransitionBundle},
    commands::{
        close::*, enter::*, look::*, map::*, movement::*, open::*, scan::*, sit::*, stand::*,
    },
    components::*,
    events::*,
    systems::*,
};

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Vec<String>>()
            .register_type::<Position>()
            .register_type::<Tile>()
            .register_type::<LifeSpawn>()
            .register_type::<DeathSpawn>()
            .register_type::<Transition>()
            .register_type::<Zone>()
            .register_type::<Door>()
            .register_type::<TileBundle>()
            .register_type::<TransitionBundle>();

        app.add_event::<MovementEvent>();

        app.add_systems(
            Update,
            (
                close,
                enter,
                look,
                map,
                movement,
                open,
                scan,
                sit,
                stand,
                on_movement_event_flee,
            ),
        );
    }
}
