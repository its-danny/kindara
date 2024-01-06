use bevy::prelude::*;

use super::systems::*;

pub struct SkillsPlugin;

impl Plugin for SkillsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (potential_regen, update_cooldowns, update_bleeding));
    }
}
