use bevy::prelude::*;

use super::{
    commands::{inventory::*, take::*},
    components::*,
};

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Item>().add_systems((inventory, take));
    }
}
