use bevy::prelude::*;

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
        app.insert_resource(PromptTimer(Timer::from_seconds(60.0, TimerMode::Repeating)));

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
