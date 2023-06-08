use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Reflect, FromReflect)]
pub struct Position(pub IVec3);

#[derive(Debug, Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Zone {
    pub name: String,
}

#[derive(Component, Reflect, FromReflect)]
pub struct Tile {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Spawn;

#[derive(Debug, Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Transition {
    pub tags: Vec<String>,
    pub zone: String,
    pub position: IVec3,
}
