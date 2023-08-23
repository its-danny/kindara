use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum RelevantStat {
    Strength,
    Dexterity,
    Intelligence,
}

#[derive(Debug, Deserialize)]
pub enum Action {
    ApplyDamage(String),
}

/// A skill definition.
#[derive(Debug, Deserialize)]
pub struct Skill {
    pub name: String,
    pub stat: RelevantStat,
    pub actions: Vec<Action>,
}

/// A collection of all available skills.
#[derive(Default, Resource)]
pub struct Skills(pub HashMap<String, Skill>);
