use bevy::prelude::*;

use super::{resources::*, systems::*};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Masteries::default());
        app.insert_resource(Skills::default());

        app.add_systems(Startup, (load_masteries, load_skills));
    }
}
