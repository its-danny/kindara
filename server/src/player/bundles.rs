use bevy::prelude::*;

use crate::keycard::Keycard;

use super::components::Character;

#[derive(Bundle)]
pub struct PlayerBundle {
    pub keycard: Keycard,
    pub character: Character,
}
