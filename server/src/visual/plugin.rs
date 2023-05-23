use bevy::prelude::*;

use super::components::Sprite;

pub struct VisualPlugin;

impl Plugin for VisualPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Sprite>();
    }
}
