use bevy::prelude::*;
use bevy_proto::prelude::*;

use crate::visual::components::Depiction;

use super::components::Npc;

#[derive(Bundle, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct NPCBundle {
    npc: Npc,
    depiction: Depiction,
}
