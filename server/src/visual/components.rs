use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Reflect, FromReflect)]
pub struct Sprite {
    pub character: String,
}

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Depiction {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub tags: Vec<String>,
}
