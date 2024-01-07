use std::fmt::{Display, Formatter};

use bevy::prelude::*;
use uuid::Uuid;

use crate::combat::events::{
    AddStatModifier, ApplyCondition, ApplyDamage, CombatLog, RemoveStatModifier, SetApproach,
    SetDistance,
};

use super::context::ExecutionContext;

#[derive(Event)]
pub struct ExecutionEvent {
    pub context: ExecutionContext,
    pub scripts: Vec<String>,
    pub phase: ExecutionPhase,
}

#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum ExecutionPhase {
    OnInit,
    OnUse,
    OnMiss,
    OnDodge,
    OnBlock,
    OnHit,
    OnEnd,
}

impl Display for ExecutionPhase {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OnInit => write!(f, "on_init"),
            Self::OnUse => write!(f, "on_use"),
            Self::OnMiss => write!(f, "on_miss"),
            Self::OnDodge => write!(f, "on_dodge"),
            Self::OnBlock => write!(f, "on_block"),
            Self::OnHit => write!(f, "on_hit"),
            Self::OnEnd => write!(f, "on_end"),
        }
    }
}

#[derive(Event, Debug)]
pub struct ActionEvent {
    pub action: Action,
    pub context: ExecutionContext,
}

#[derive(Event, Debug)]
pub struct ProcessEvent {
    pub context: ExecutionContext,
}

#[derive(Clone, Debug)]
pub enum Action {
    ApplyDamage(ApplyDamage),
    SetDistance(SetDistance),
    SetApproach(SetApproach),
    ApplyCondition(ApplyCondition),
    AddStatModifier(AddStatModifier),
    RemoveStatModifier(RemoveStatModifier),
    CombatLog(CombatLog),
    SendMessage(SendMessage),
}

#[derive(Clone, Debug)]
pub struct SendMessage {
    pub target: Entity,
    pub message: String,
}

#[derive(Event)]
pub struct ApplyDamageResponse {
    pub context: ExecutionContext,
    pub callback_id: Uuid,
    pub damage: u32,
    pub kind: String,
    pub crit: bool,
}
