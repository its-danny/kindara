use bevy::prelude::*;
use bevy_proto::prelude::*;

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
                ..input.stats.clone()
            };

            entity.insert(stats);
        }
    }

    fn remove(_input: &Self::Input, context: &mut SchematicContext) {
        if let Some(mut entity) = context.entity_mut() {
            entity.remove::<Stats>();
        }
    }
}
