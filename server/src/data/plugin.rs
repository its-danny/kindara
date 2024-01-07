use bevy::prelude::*;

use super::{resources::*, systems::*};

pub struct DataPlugin;

impl Plugin for DataPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(DamageKinds::default());
        app.insert_resource(Resistances::default());
        app.insert_resource(Masteries::default());
        app.insert_resource(Skills::default());
        app.insert_resource(Conditions::default());

        app.add_systems(
            Startup,
            (
                load_damage_kinds,
                load_resistances,
                load_masteries,
                load_skills,
                load_conditions,
            ),
        );
    }
}
