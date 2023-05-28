use bevy::prelude::*;

use super::commands::{say::*, who::*};

pub struct SocialPlugin;

impl Plugin for SocialPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((say, who));
    }
}
