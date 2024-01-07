use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;
use strum_macros::{Display, EnumIter};

use crate::combat::components::Distance;

#[derive(Debug, Deserialize)]
pub struct DamageKind {
    pub id: String,
    pub name: String,
    pub description: String,
    pub resistances: Vec<String>,
}

#[derive(Default, Resource)]
pub struct DamageKinds(pub HashMap<String, DamageKind>);

#[derive(Debug, Deserialize)]
pub struct Resistance {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Default, Resource)]
pub struct Resistances(pub HashMap<String, Resistance>);

/// A mastery definition.
#[derive(Debug, Deserialize)]
pub struct Mastery {
    pub id: String,
    pub name: String,
    pub vitality: u32,
    pub stamina: u32,
    pub strength: u32,
    pub dexterity: u32,
    pub intelligence: u32,
    pub auto_attack: String,
    pub skills: Vec<String>,
}

/// A collection of all masteries.
#[derive(Default, Resource)]
pub struct Masteries(pub HashMap<String, Mastery>);

/// A skill definition.
#[derive(Debug, Deserialize, Clone)]
pub struct Skill {
    pub id: String,
    pub commands: Vec<String>,
    pub name: String,
    pub description: String,
    pub cost: u32,
    pub cooldown: u32,
    pub distance: Distance,
    pub dodge_difficulty: f32,
    pub block_difficulty: f32,
    pub scripts: Vec<String>,
}

/// A collection of all available skills.
#[derive(Default, Resource)]
pub struct Skills(pub HashMap<String, Skill>);

#[derive(Debug, Clone, Reflect, PartialEq, EnumIter, Display)]
pub enum Stat {
    Vitality,
    Stamina,
    Strength,
    Dexterity,
    Intelligence,
    Health,
    Vigor,
    VigorRegen,
    DodgeChance,
    DodgeRate,
    BlockChance,
    BlockRate,
    Fleet,
    AttackSpeed,
    Dominance,
    CritStrikeChance,
    CritStrikeDamage,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Condition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub scripts: Vec<String>,
}

#[derive(Default, Resource)]
pub struct Conditions(pub HashMap<String, Condition>);
