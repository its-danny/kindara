use bevy::prelude::*;
use bevy_proto::prelude::*;

use super::components::{Cooldowns, PotentialRegenTimer, Stats};

#[derive(Bundle, Schematic, Reflect, Default)]
#[reflect(Schematic)]
pub struct CombatBundle {
    pub stats: Stats,
    #[reflect(default)]
    pub cooldowns: Cooldowns,
    #[reflect(default)]
    pub potential_regen_timer: PotentialRegenTimer,
}
