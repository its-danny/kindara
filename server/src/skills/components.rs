use bevy::prelude::*;

#[derive(Component)]
pub struct Bleeding {
    pub source: Entity,
    pub tick: Timer,
    pub duration: Timer,
    pub roll: String,
}
