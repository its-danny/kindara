use bevy::prelude::*;

use super::components::InMenu;

pub fn remove_menu_if_changed_tiles(
    mut commands: Commands,
    mut query: Query<Entity, (With<InMenu>, Changed<Parent>)>,
) {
    for entity in query.iter_mut() {
        commands.entity(entity).remove::<InMenu>();
    }
}
