use bevy::prelude::*;

use super::{
    commands::{drop::*, inventory::*, place::*, take::*},
    components::*,
};

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Item>()
            .register_type::<CanTake>()
            .register_type::<CanPlace>()
            .register_type::<Surface>()
            .register_type::<SurfaceKind>()
            .register_type::<Size>()
            .add_systems((inventory, take, drop, place));
    }
}
