use bevy::prelude::*;

use super::commands::{chat::*, emote::*, say::*, who::*, yell::*};

pub struct SocialPlugin;

impl Plugin for SocialPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((chat, emote, say, who, yell));
    }
}
