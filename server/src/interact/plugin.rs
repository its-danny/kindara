use bevy::prelude::*;

use super::{
    commands::{examine::*, place::*, quit::*, roll::*, take::*},
    components::*,
    systems::*,
};

pub struct InteractPlugin;

impl Plugin for InteractPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Interaction>()
            .register_type::<Vec<Interaction>>()
            .register_type::<Interactions>();

        app.add_systems(
            Update,
            (
                examine,
                place,
                quit,
                remove_menu_if_changed_tiles,
                roll,
                take,
            ),
        );
    }
}
