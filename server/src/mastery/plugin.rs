use bevy::prelude::*;

use super::{load::load_masteries, resources::Masteries};

pub struct MasteryPlugin;

impl Plugin for MasteryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Masteries::default());

        app.add_systems(Startup, load_masteries);
    }
}
