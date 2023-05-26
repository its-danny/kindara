use bevy::prelude::*;

use super::commands::config::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((config, handle_save_config_task));
    }
}
