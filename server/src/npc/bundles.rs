use bevy::prelude::*;
use bevy_proto::prelude::*;

use crate::visual::components::Depiction;

use super::components::Npc;

#[derive(Bundle, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct NpcBundle {
    pub npc: Npc,
    pub depiction: Depiction,
}
