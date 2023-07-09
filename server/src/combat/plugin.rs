use bevy::prelude::*;

use super::{bundles::*, commands::attack::*, components::*};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Attributes>();
        app.register_type::<CombatBundle>();

        app.add_system(attack);
    }
}
