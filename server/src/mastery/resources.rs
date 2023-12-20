use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;

/// A mastery definition.
#[derive(Debug, Deserialize)]
pub struct Mastery {
    pub id: String,
    pub name: String,
    pub vitality: u32,
    pub proficiency: u32,
    pub speed: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub skills: Vec<String>,
}

/// A collection of all masteries.
#[derive(Default, Resource)]
pub struct Masteries(pub HashMap<String, Mastery>);
