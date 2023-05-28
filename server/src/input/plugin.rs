use bevy::prelude::*;

use crate::Set;

use super::{events::*, systems::*};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ParsedCommand>()
            .add_system(parse_command.in_base_set(Set::Input));
    }
}
