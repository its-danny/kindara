use bevy::prelude::*;

use crate::{combat::bundles::CombatBundle, keycard::Keycard};

use super::components::Character;

#[derive(Bundle)]
pub struct PlayerBundle {
    pub keycard: Keycard,
    pub character: Character,
    pub combat: CombatBundle,
}
