use bevy::prelude::*;

use super::components::*;

pub struct InteractPlugin;

impl Plugin for InteractPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Interaction>()
            .register_type::<Vec<Interaction>>()
            .register_type::<Interactions>();
    }
}
