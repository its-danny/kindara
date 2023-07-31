use bevy::prelude::*;
use bevy_proto::prelude::*;

use crate::visual::components::{Depiction, Sprite};

use super::components::{Position, Tile, Transition};

#[derive(Bundle, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct TileBundle {
    pub tile: Tile,
    pub sprite: Sprite,
    pub position: Position,
}

#[derive(Bundle, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct TransitionBundle {
    pub transition: Transition,
    pub depiction: Depiction,
}
