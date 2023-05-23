use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Position {
    pub zone: Zone,
    pub coords: IVec3,
}

#[derive(PartialEq, Reflect, FromReflect)]
pub enum Zone {
    Void,
    Movement,
}

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Tile {
    pub name: String,
    pub description: String,
}

#[derive(Component)]
pub struct Impassable;
