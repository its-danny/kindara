use bevy::prelude::*;

use crate::spatial::components::Position;

use super::components::Character;

#[derive(Bundle)]
pub struct PlayerBundle {
    pub character: Character,
    pub position: Position,
}
