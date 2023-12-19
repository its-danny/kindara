use bevy::prelude::*;

use super::{load::load_skills, resources::Skills, systems::potential_regen};

pub struct SkillsPlugin;

impl Plugin for SkillsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Skills::default());

        app.add_systems(Startup, load_skills);
        app.add_systems(Update, potential_regen);
    }
}
