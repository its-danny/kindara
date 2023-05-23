use bevy::prelude::*;

use super::commands::*;

pub struct SpatialPlugin;

impl Plugin for SpatialPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(map);
        app.add_system(movement);
        app.add_system(teleport);
    }
}
