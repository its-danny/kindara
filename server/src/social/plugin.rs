use bevy::prelude::*;

use super::commands::*;

pub struct SocialPlugin;

impl Plugin for SocialPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(who);
    }
}
