use bevy::prelude::*;

use super::{bundles::*, commands::attack::*, components::*, systems::*};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Stats>();
        app.register_type::<Attributes>();
        app.register_type::<Status>();
        app.register_type::<Defense>();
        app.register_type::<Resistance>();
        app.register_type::<Offense>();
        app.register_type::<Cooldowns>();
        app.register_type::<PotentialRegenTimer>();
        app.register_type::<CombatBundle>();

        app.add_systems(
            Update,
            (
                attack,
                update_attack_timer,
                on_hostile_death,
                on_player_death,
            ),
        );
    }
}
