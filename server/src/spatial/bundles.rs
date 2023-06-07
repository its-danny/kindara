use bevy::prelude::*;
use bevy_proto::prelude::*;

use crate::visual::components::Sprite;

use super::components::{Position, Tile};

#[derive(Bundle, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct TileBundle {
    pub tile: Tile,
    pub sprite: Sprite,
    pub position: Position,
}
