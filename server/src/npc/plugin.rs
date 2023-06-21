use bevy::prelude::*;

use super::{bundles::*, components::*};

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<NpcBundle>().register_type::<Npc>();
    }
}
