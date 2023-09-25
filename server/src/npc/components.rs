use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Reflect)]
pub struct Npc {
    pub skills: Vec<String>,
}

#[derive(Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct EnemySpawner {
    pub delay: f32,
    pub enemies: (String, u16, u16),
    pub spawned: Vec<Entity>,
}

#[derive(Component)]
pub struct SpawnTimer(pub Timer);
