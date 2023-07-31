use bevy::prelude::*;

use super::systems::*;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, on_network_event);
    }
}
