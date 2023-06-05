use std::fmt::{Display, Formatter};

use bevy::prelude::*;

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, Reflect, FromReflect)]
pub enum Zone {
    Void,
    Movement,
}

#[derive(Component, Reflect, FromReflect)]
pub struct Position {
    pub zone: Zone,
    pub coords: IVec3,
}

impl Display for Zone {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Zone::Void => write!(f, "The Void"),
            Zone::Movement => write!(f, "Testing - Movement"),
        }
    }
}

#[derive(Component, Reflect, FromReflect)]
pub struct Tile {
    pub name: String,
    pub description: String,
}

#[derive(Component, Reflect, FromReflect)]
pub struct Transition {
    pub tags: Vec<String>,
    pub zone: Zone,
    pub coords: IVec3,
}
