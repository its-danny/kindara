use bevy::prelude::*;

use super::{bundles::*, components::*, systems::*};

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Vec<Entity>>()
            .register_type::<(String, u16, u16)>()
            .register_type::<Npc>()
            .register_type::<Friendly>()
            .register_type::<Hostile>()
            .register_type::<NpcBundle>()
            .register_type::<FriendlyBundle>()
            .register_type::<HostileBundle>()
            .register_type::<HostileSpawner>();

        app.add_systems(Update, handle_hostile_spawner);
    }
}
