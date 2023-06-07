use bevy::prelude::*;

use super::components::Character;

#[derive(Bundle)]
pub struct PlayerBundle {
    pub character: Character,
}
