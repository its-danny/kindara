use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;

use crate::combat::components::Distance;

#[derive(Debug, Deserialize)]
pub enum RelevantStat {
    Strength,
    Dexterity,
    Intelligence,
}

#[derive(Debug, Deserialize)]
pub enum DamageType {
    Physical,
}

#[derive(Debug, Deserialize)]
pub enum StatusEffect {
    Bleeding,
}

#[derive(Debug, Deserialize)]
pub enum Action {
    ApplyDamage(String),
    // Effect, Roll, Tick, Duration
    ApplyStatus(StatusEffect, String, u32, u32),
}

/// A skill definition.
#[derive(Debug, Deserialize)]
pub struct Skill {
    pub name: String,
    pub flavor: String,
    pub commands: Vec<String>,
    pub stat: RelevantStat,
    pub damage_type: DamageType,
    pub difficulty: u32,
    pub distance: Distance,
    pub cost: u32,
    pub cooldown: u32,
    pub actions: Vec<Action>,
}

/// A collection of all available skills.
#[derive(Default, Resource)]
pub struct Skills(pub HashMap<String, Skill>);
