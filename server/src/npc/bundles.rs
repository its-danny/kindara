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
                        status: Status {
                            health: input.combat.stats.max_health(),
                            vigor: input.combat.stats.max_vigor(),
                            ..Default::default()
                        },
                        ..input.combat.stats.clone()
                    },
                    cooldowns: input.combat.cooldowns.clone(),
                    conditions: input.combat.conditions.clone(),
                    modifiers: input.combat.modifiers.clone(),
                    health_regen_timer: input.combat.health_regen_timer.clone(),
                    vigor_regen_timer: input.combat.vigor_regen_timer.clone(),
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
