use bevy::prelude::*;

use crate::values::PROMPT_TICK;

use super::{
    commands::{config::*, describe::*},
    events::Prompt,
    resources::PromptTimer,
    systems::*,
};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Prompt>();
        app.insert_resource(PromptTimer(Timer::from_seconds(
            PROMPT_TICK,
            TimerMode::Repeating,
        )));

        app.add_systems(
            Update,
            (
                config,
                handle_save_config_task,
                describe,
                handle_save_description_task,
                send_prompt,
                send_prompt_on_timer,
            ),
        );

        app.add_systems(Update, handle_client_width);
    }
}
