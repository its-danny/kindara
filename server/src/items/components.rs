use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Item {
    pub name: String,
    pub name_on_ground: String,
    pub description: String,
}
