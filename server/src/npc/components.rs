use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Reflect)]
pub struct Npc;

#[derive(Component, Reflect, Clone)]
pub struct Hostile {
    pub skills: Vec<String>,
}

#[derive(Component, Reflect)]
pub struct Friendly;

#[derive(Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct HostileSpawner {
    pub delay: f32,
    pub hostiles: (String, u16, u16),
    pub spawned: Vec<Entity>,
}

#[derive(Component)]
pub struct HostileSpawnTimer(pub Timer);
