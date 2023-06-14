use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource)]
pub struct SaveTimer(pub Timer);

#[derive(Debug, Default, Serialize, Deserialize, Resource)]
pub struct WorldState {
    pub characters: Vec<WorldStateCharacter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateCharacter {
    pub id: i64,
    pub tile: String,
    pub inventory: Vec<String>,
}
