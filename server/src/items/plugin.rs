use bevy::prelude::*;

use super::components::*;

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Item>();
    }
}
