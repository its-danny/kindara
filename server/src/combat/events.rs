use bevy::prelude::*;
use uuid::Uuid;

use crate::{
    data::resources::{Condition, Skill, Stat},
    lua::{context::ExecutionContext, events::ExecutionPhase},
};

use super::components::{Approach, Distance};

#[derive(Event, Debug)]
pub struct CombatEvent {
    pub source: Entity,
    pub trigger: CombatEventTrigger,
    pub kind: CombatEventKind,
}

#[derive(Clone, Debug)]
pub enum CombatEventTrigger {
    Skill(Skill),
    Condition(Condition),
    Movement,
}

#[derive(Debug)]
pub enum CombatEventKind {
    Attack,
    AttemptHit,
    AttemptDodge,
    AttemptBlock,
    ExecuteScripts(ExecutionPhase),
    ExecuteCondition(ExecutionPhase),
    AttemptFlee(String),
    ApplyDamage(ApplyDamage),
    ApplyCondition(ApplyCondition),
    SetDistance(SetDistance),
    SetApproach(SetApproach),
    AddStatModifier(AddStatModifier),
    RemoveStatModifier(RemoveStatModifier),
    CombatLog(CombatLog),
}

#[derive(Clone, Debug)]
pub struct ApplyDamage {
    pub target: Entity,
    pub damage: f32,
    pub kind: String,
    pub with_callback: Option<WithCallback>,
}

#[derive(Clone, Debug)]
pub struct ApplyCondition {
    pub target: Entity,
    pub condition: String,
    pub duration: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct SetDistance {
    pub target: Entity,
    pub distance: Distance,
}

#[derive(Clone, Debug)]
pub struct SetApproach {
    pub target: Entity,
    pub approach: Approach,
}

#[derive(Clone, Debug)]
pub struct AddStatModifier {
    pub target: Entity,
    pub id: String,
    pub stat: Stat,
    pub amount: f32,
}

#[derive(Clone, Debug)]
pub struct RemoveStatModifier {
    pub target: Entity,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct CombatLog {
    pub source: Entity,
    pub target: Entity,
    pub kind: CombatLogKind,
}

#[derive(Clone, Debug)]
pub enum CombatLogKind {
    Used(Used),
    Missed(Missed),
    Dodged(Dodged),
    Blocked(Blocked),
    Damaged(Damaged),
    ConditionApplied(ConditionApplied),
    ConditionRemoved(ConditionRemoved),
}

#[derive(Debug, Clone)]
pub struct Used {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Missed {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Dodged {
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct Blocked {
    pub message: String,
    pub damage: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct Damaged {
    pub message: String,
    pub damage: u32,
    pub kind: String,
    pub crit: bool,
}

#[derive(Debug, Clone)]
pub struct ConditionApplied {
    pub message: String,
    pub condition: String,
}

#[derive(Debug, Clone)]
pub struct ConditionRemoved {
    pub message: String,
    pub condition: String,
}

#[derive(Clone, Debug)]
pub struct WithCallback {
    pub context: ExecutionContext,
    pub callback_id: Uuid,
}
