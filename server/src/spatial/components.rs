use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Reflect)]
pub struct Position(pub IVec3);

#[derive(Debug, Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct Zone {
    pub name: String,
}

#[derive(Component, Reflect)]
pub struct Tile {
    pub name: String,
    pub description: String,
}

/// A marker component that indicates an entity is a spawn point.
#[derive(Debug, Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct Spawn;

/// A component that marks an entity as a transition to another zone.
#[derive(Debug, Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct Transition {
    pub zone: String,
    pub position: IVec3,
}

/// A component that marks an entity as seated. The string is the
/// phrase we use to describe the entity's position, e.g. "on the
/// couch" or "in the chair".
#[derive(Component)]
pub struct Seated(pub String);
