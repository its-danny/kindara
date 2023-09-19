use bevy::prelude::*;
use bevy_nest::prelude::*;

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
    pub mastery: String,
}

#[derive(Component)]
pub struct Online;
