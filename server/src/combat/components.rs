use bevy::prelude::*;

use crate::input::events::ParsedCommand;

/// The base attributes of an entity that can do combat.
#[derive(Component, Reflect, Clone)]
pub struct Attributes {
    /// Determines base health, max health, and health regeneration amount.
    pub vitality: u32,
    /// Determines base damage, max potential, and potential regeneration amount.
    pub proficiency: u32,
    /// Determines attack speed and movement speed.
    pub speed: u32,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            vitality: 10,
            proficiency: 5,
            speed: 3,
        }
    }
}

impl Attributes {
    pub fn max_health(&self) -> u32 {
        self.vitality * 10
    }

    #[allow(dead_code)]
    pub fn base_damage(&self) -> u32 {
        self.proficiency * 2
    }
}

/// The current state of an entity that can do combat.
#[derive(Component)]
pub struct State {
    pub health: u32,
}

impl State {
    /// Applies damage to the entity's health, saturating at 0.
    pub fn apply_damage(&mut self, damage: i32) {
        self.health = self.health.saturating_sub(damage as u32);
    }
}

/// Added to an entity when it has attacked to prevent acting faster
/// than their attack speed. The timer is handled via the `update_attack_timer` system.
#[derive(Component)]
pub struct HasAttacked {
    pub timer: Timer,
}

#[derive(Component)]
pub struct QueuedAttack(pub ParsedCommand);
