use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;

use crate::{combat::components::Distance, skills::resources::StatusEffect};

/// A mastery definition.
#[derive(Debug, Deserialize)]
pub struct Mastery {
    pub id: String,
    pub name: String,
    pub vitality: u32,
    pub proficiency: u32,
    pub attack_speed: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub skills: Vec<String>,
}

/// A collection of all masteries.
#[derive(Default, Resource)]
pub struct Masteries(pub HashMap<String, Mastery>);

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
pub enum Action {
    ApplyDamage(String),
    // Effect, Roll, Tick, Duration
    ApplyStatus(StatusEffect, String, u32, u32),
}

/// A skill definition.
#[derive(Debug, Deserialize)]
pub struct Skill {
    pub id: String,
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
