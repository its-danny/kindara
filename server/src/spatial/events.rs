use bevy::prelude::*;

#[derive(Event)]
pub struct MovementEvent {
    pub source: Entity,
    pub kind: MovementEventKind,
}

pub enum MovementEventKind {
    Flee(String),
}
