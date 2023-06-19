use bevy::prelude::*;

use super::{
    bundles::ItemBundle,
    commands::{drop::*, inventory::*, place::*, take::*},
    components::*,
};

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ItemBundle>()
            .register_type::<Item>()
            .register_type::<Surface>()
            .register_type::<SurfaceKind>()
            .register_type::<Size>();

        app.add_systems((inventory, take, drop, place));
    }
}
