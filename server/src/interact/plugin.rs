use bevy::prelude::*;

use super::{
    commands::{examine::*, place::*, take::*},
    components::*,
    systems::*,
};

pub struct InteractPlugin;

impl Plugin for InteractPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Interaction>()
            .register_type::<Vec<Interaction>>()
            .register_type::<Interactions>();

        app.add_systems(Update, (examine, take, place, remove_menu_if_changed_tiles));
    }
}
