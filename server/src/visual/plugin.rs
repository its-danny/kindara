use bevy::prelude::*;

use super::components::{Depiction, Sprite};

pub struct VisualPlugin;

impl Plugin for VisualPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Sprite>().register_type::<Depiction>();
    }
}
