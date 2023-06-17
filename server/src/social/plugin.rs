use bevy::prelude::*;

use super::commands::{announce::*, chat::*, emote::*, say::*, who::*, yell::*};

pub struct SocialPlugin;

impl Plugin for SocialPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((announce, chat, emote, say, who, yell));
    }
}
