use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Reflect, FromReflect)]
pub struct Npc;

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct EnemySpawner {
    pub delay: f32,
    pub enemies: (String, u16, u16),
    pub spawned: Vec<Entity>,
}

#[derive(Component)]
pub struct SpawnTimer(pub Timer);
