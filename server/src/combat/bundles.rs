use bevy::prelude::*;
use bevy_proto::prelude::*;

use crate::skills::components::PotentialRegenTimer;

use super::components::Stats;

#[derive(Bundle, Reflect)]
#[reflect(Schematic)]
pub struct CombatBundle {
    pub stats: Stats,
}

impl Schematic for CombatBundle {
    type Input = Self;

    fn apply(input: &Self::Input, context: &mut SchematicContext) {
        if let Some(mut entity) = context.entity_mut() {
            let stats = Stats {
                health: input.stats.max_health(),
                potential: input.stats.max_potential(),
                ..input.stats.clone()
            };

            entity.insert(stats);

            entity.insert(PotentialRegenTimer(Timer::from_seconds(
                1.0,
                TimerMode::Repeating,
            )));
        }
    }

    fn remove(_input: &Self::Input, context: &mut SchematicContext) {
        if let Some(mut entity) = context.entity_mut() {
            entity.remove::<Stats>();
        }
    }
}
