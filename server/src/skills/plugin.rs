use bevy::prelude::*;

use super::{load::load_skills, resources::Skills};

pub struct SkillsPlugin;

impl Plugin for SkillsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Skills::default());

        app.add_startup_systems((load_skills,));
    }
}
