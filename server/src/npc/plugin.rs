use bevy::prelude::*;

use super::{bundles::*, components::*, systems::*};

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Vec<Entity>>()
            .register_type::<(String, u16, u16)>()
            .register_type::<NpcBundle>()
            .register_type::<Npc>()
            .register_type::<EnemySpawner>();

        app.add_systems(
            Update,
            (handle_enemy_spawner, on_enter_combat, attack_when_able),
        );
    }
}
