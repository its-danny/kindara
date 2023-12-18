use bevy::prelude::*;

use super::commands::menu::menu;

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, menu);
    }
}
