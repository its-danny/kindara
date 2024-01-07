use bevy::prelude::*;
use mlua::prelude::*;

use super::{events::*, resources::*, systems::*};

pub struct LuaPlugin;

impl Plugin for LuaPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Scripts::default());
        app.insert_non_send_resource(Lua::new());

        app.add_event::<ExecutionEvent>();
        app.add_event::<ActionEvent>();
        app.add_event::<ProcessEvent>();
        app.add_event::<ApplyDamageResponse>();

        app.add_systems(Startup, load_scripts);
        app.add_systems(
            Update,
            (
                execute_scripts,
                process_action_events,
                handle_combat_event_action,
                handle_send_message_action,
                on_apply_damage_response,
            ),
        );
    }
}
