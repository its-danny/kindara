use bevy::prelude::*;

use super::systems::*;

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(on_network_event);
        app.add_system(authenticate);
        app.add_system(handle_user_exists_task);
        app.add_system(handle_authenticate_task);
    }
}
