use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Reflect)]
pub struct Position(pub IVec3);

/// A zone is a collection of tiles that make up a single area of the world.
#[derive(Debug, Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct Zone {
    pub name: String,
}

/// A tile is a single block of the world.
/// When moving, the player becomes a child of the tile they are moving to.
#[derive(Component, Reflect)]
pub struct Tile {
    pub name: String,
    pub description: String,
}

/// A marker component that indicates an entity is a spawn point for
/// a living player.
#[derive(Debug, Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct LifeSpawn;

/// A marker component that indicates an entity is a spawn point for
/// after a player dies.
#[derive(Debug, Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct DeathSpawn;

/// A component that marks an entity as a transition to another zone.
#[derive(Debug, Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct Transition {
    pub zone: String,
    pub position: IVec3,
}

/// A component that marks an entity as performing an action. The string is the
/// phrase we use to describe the entity's action, e.g. "is sitting" or
/// "is fighting a goblin."
#[derive(Component)]
pub struct Action(pub String);

/// A marker component that indicates an entity is sitting.
#[derive(Component)]
pub struct Seated;

/// A component that marks an entity as a door. The blocks field is the
/// direction the door blocks, e.g. (0, 1, 0) means the door blocks the
/// tile to the south.
#[derive(Debug, Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct Door {
    pub blocks: IVec3,
    pub is_open: bool,
}
