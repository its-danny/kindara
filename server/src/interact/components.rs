use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(PartialEq, Reflect, FromReflect)]
pub enum Interaction {
    Take,
    Place,
}

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Interactions(pub Vec<Interaction>);
