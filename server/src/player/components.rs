use bevy::prelude::*;
use bevy_nest::server::ClientId;

use super::config::CharacterConfig;

#[derive(Debug, Component)]
pub struct Client {
    pub id: ClientId,
    pub width: u16,
}

#[derive(Component)]
pub struct Character {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub config: CharacterConfig,
    pub state: CharacterState,
}

pub enum CharacterState {
    Idle,
}

#[derive(Component)]
pub struct Online;
