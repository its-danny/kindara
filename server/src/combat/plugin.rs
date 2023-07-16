use bevy::prelude::*;

use super::{bundles::*, commands::attack::*, components::*, systems::*};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Attributes>();
        app.register_type::<CombatBundle>();

        app.add_systems((attack, update_attack_timer));
    }
}
