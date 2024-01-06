use bevy::prelude::*;
use bevy_proto::prelude::*;

use crate::{
    combat::{
        bundles::CombatBundle,
        components::{Stats, Status},
    },
    interact::components::Interactions,
    visual::components::Depiction,
};

use super::components::{Hostile, Npc};

#[derive(Bundle, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct NpcBundle {
    pub npc: Npc,
    pub depiction: Depiction,
    pub interactions: Interactions,
}

#[derive(Bundle, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct FriendlyBundle {}

#[derive(Bundle, Reflect)]
#[reflect(Schematic)]
pub struct HostileBundle {
    pub hostile: Hostile,
    pub combat: CombatBundle,
}

impl Schematic for HostileBundle {
    type Input = Self;

    fn apply(input: &Self::Input, context: &mut SchematicContext) {
        if let Some(mut entity) = context.entity_mut() {
            entity.insert((
                input.hostile.clone(),
                CombatBundle {
                    stats: Stats {
                        state: Status {
                            health: input.combat.stats.max_health(),
                            potential: input.combat.stats.max_potential(),
                            ..Default::default()
                        },
                        ..input.combat.stats.clone()
                    },
                    cooldowns: input.combat.cooldowns.clone(),
                    potential_regen_timer: input.combat.potential_regen_timer.clone(),
                },
            ));
        }
    }

    fn remove(_input: &Self::Input, context: &mut SchematicContext) {
        if let Some(mut entity) = context.entity_mut() {
            entity.remove::<Self>();
        }
    }
}
