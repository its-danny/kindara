use bevy::prelude::*;
use bevy_nest::server::ClientId;

#[derive(Component)]
pub struct Client(pub ClientId);

#[derive(Component)]
pub struct Character {
    pub name: String,
}
