use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub enum Action {
    ApplyDamage(i32),
}

#[derive(Debug, Deserialize)]
pub struct Skill {
    pub name: String,
    pub actions: Vec<Action>,
}

#[derive(Default, Resource)]
pub struct Skills(pub HashMap<String, Skill>);
