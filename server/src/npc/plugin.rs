use bevy::prelude::*;

use super::{bundles::*, components::*};

pub struct NPCPlugin;

impl Plugin for NPCPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<NPCBundle>().register_type::<Npc>();
    }
}
