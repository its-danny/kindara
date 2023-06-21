use bevy::prelude::*;

use super::commands::attack::*;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(attack);
    }
}
