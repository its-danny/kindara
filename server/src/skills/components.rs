use bevy::{prelude::*, utils::HashMap};

#[derive(Component)]
pub struct PotentialRegenTimer(pub Timer);

#[derive(Component, Default)]
pub struct Cooldowns(pub HashMap<String, Timer>);

#[derive(Component)]
pub struct Bleeding {
    pub source: Entity,
    pub tick: Timer,
    pub length: Timer,
    pub roll: String,
}
