use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum Action {
    ApplyDamage(i32),
}

/// A skill definition.
#[derive(Debug, Deserialize)]
pub struct Skill {
    pub name: String,
    pub actions: Vec<Action>,
}

/// A collection of all available skills.
#[derive(Default, Resource)]
pub struct Skills(pub HashMap<String, Skill>);
