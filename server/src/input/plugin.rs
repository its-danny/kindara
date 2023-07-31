use bevy::prelude::*;

use super::{events::*, systems::*};

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ParsedCommand>().add_event::<ProxyCommand>();

        app.add_systems(First, (parse_command, handle_proxy_command));
    }
}
