use bevy::prelude::*;
use bevy_proto::prelude::*;

use super::components::{
    Conditions, Cooldowns, HealthRegenTimer, Modifiers, Stats, VigorRegenTimer,
};

#[derive(Bundle, Schematic, Reflect, Default)]
#[reflect(Schematic)]
pub struct CombatBundle {
    pub stats: Stats,
    #[reflect(default)]
    pub cooldowns: Cooldowns,
    #[reflect(default)]
    pub conditions: Conditions,
    #[reflect(default)]
    pub modifiers: Modifiers,
    #[reflect(default)]
    pub health_regen_timer: HealthRegenTimer,
    #[reflect(default)]
    pub vigor_regen_timer: VigorRegenTimer,
}
