use bevy::prelude::*;
use bevy_proto::prelude::*;

use super::components::{Attributes, State};

#[derive(Bundle, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct CombatBundle {
    pub attributes: Attributes,
}

impl Schematic for CombatBundle {
    type Input = Self;

    fn apply(input: &Self::Input, context: &mut SchematicContext) {
        if let Some(mut entity) = context.entity_mut() {
            entity.insert(input.attributes.clone());

            entity.insert(State {
                health: input.attributes.max_health(),
            });
        }
    }

    fn remove(_input: &Self::Input, context: &mut SchematicContext) {
        if let Some(mut entity) = context.entity_mut() {
            entity.remove::<Attributes>();
            entity.remove::<State>();
        }
    }
}
