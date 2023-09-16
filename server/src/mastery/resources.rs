use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;

/// A mastery definition.
#[derive(Component, Debug, Deserialize)]
pub struct Mastery {
    pub name: String,
    pub skills: Vec<String>,
}

/// A collection of all masteries.
#[derive(Default, Resource)]
pub struct Masteries(pub HashMap<String, Mastery>);
