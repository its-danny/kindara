use bevy::prelude::*;

use super::{
    commands::{config::*, describe::*},
    systems::*,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            config,
            handle_save_config_task,
            describe,
            handle_save_description_task,
        ))
        .add_system(handle_client_width);
    }
}
