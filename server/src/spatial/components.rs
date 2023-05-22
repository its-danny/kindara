use bevy::prelude::*;

#[derive(Component)]
pub struct Position(pub IVec3);

#[derive(Component)]
pub struct Tile {
    pub name: String,
    pub description: String,
}

#[derive(Component)]
pub struct Impassable;
