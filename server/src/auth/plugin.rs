use bevy::prelude::*;

use super::systems::*;

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                authenticate,
                handle_user_exists_task,
                handle_authenticate_task,
            ),
        );
    }
}
