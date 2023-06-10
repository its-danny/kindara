use bevy::prelude::*;

use super::{
    commands::{drop::*, inventory::*, take::*},
    components::*,
};

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Item>()
            .register_type::<CanTake>()
            .register_type::<CanPlace>()
            .register_type::<Surface>()
            .register_type::<SurfaceType>()
            .register_type::<PlacementSize>()
            .register_type::<Size>()
            .add_systems((inventory, take, drop));
    }
}
