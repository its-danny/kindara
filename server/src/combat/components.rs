use std::fmt::{self, Display, Formatter};

use bevy::prelude::*;
use caith::Roller;
use serde::Deserialize;

use crate::{
    input::events::ParsedCommand,
    skills::resources::{Action, RelevantStat, Skill},
};

/// The base attributes of an entity that can do combat.
#[derive(Component, Reflect, Clone)]
pub struct Attributes {
    /// Determines base health, max health, and health regeneration amount.
    pub vitality: u32,
    /// Determines base damage, max potential, and potential regeneration amount.
    pub proficiency: u32,
    /// Determines attack speed.
    pub speed: u32,
    /// Modifier for brute force attacks.
    pub strength: u32,
    /// Modifier for finesse attacks.
    pub dexterity: u32,
    /// Modifier for magic attacks.
    pub intelligence: u32,
    /// How likely an entity is to flee from you.
    pub dominance: u32,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            vitality: 10,
            proficiency: 5,
            speed: 3,
            strength: 0,
            dexterity: 0,
            intelligence: 0,
            dominance: 0,
        }
    }
}

impl Attributes {
    pub fn max_health(&self) -> u32 {
        self.vitality * 10
    }
}

/// The current state of an entity that can do combat.
#[derive(Component)]
pub struct State {
    pub health: u32,
}

impl State {
    /// Applies damage to the entity's health, saturating at 0.
    pub fn apply_damage(&mut self, damage: u32) {
        self.health = self.health.saturating_sub(damage);
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub enum Distance {
    Near,
    Far,
}

impl Display for Distance {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Distance::Near => write!(f, "near"),
            Distance::Far => write!(f, "far"),
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct InCombat {
    pub target: Entity,
    pub distance: Distance,
}

pub enum HitError {
    Missed,
}

impl InCombat {
    /// Attacks the target entity with the given skill.
    pub fn attack(
        &self,
        bevy: &mut Commands,
        attacker: Entity,
        skill: &Skill,
        attacker_attributes: &Attributes,
        target_state: &mut State,
    ) -> Result<(), HitError> {
        bevy.entity(attacker).insert(HasAttacked {
            timer: Timer::from_seconds(attacker_attributes.speed as f32, TimerMode::Once),
        });

        match self.roll_hit() {
            Ok(_) => {
                self.apply_actions(skill, attacker_attributes, target_state);

                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    fn roll_hit(&self) -> Result<(), HitError> {
        let roller = Roller::new("2d10").unwrap();
        let roll = roller.roll().unwrap();
        let quality = roll.as_single().unwrap().get_total();

        let roller = Roller::new("1d10").unwrap();
        let roll = roller.roll().unwrap();
        let dodge = roll.as_single().unwrap().get_total();

        if quality < dodge {
            Err(HitError::Missed)
        } else {
            Ok(())
        }
    }

    fn apply_actions(
        &self,
        skill: &Skill,
        attacker_attributes: &Attributes,
        target_state: &mut State,
    ) {
        for action in &skill.actions {
            match action {
                Action::ApplyDamage(roll) => {
                    let roller = Roller::new(roll).unwrap();
                    let roll = roller.roll().unwrap();
                    let mut damage = roll.as_single().unwrap().get_total() as u32;

                    damage += match &skill.stat {
                        RelevantStat::Strength => attacker_attributes.strength,
                        RelevantStat::Dexterity => attacker_attributes.dexterity,
                        RelevantStat::Intelligence => attacker_attributes.intelligence,
                    };

                    target_state.apply_damage(damage);
                }
            }
        }
    }

    // You can move if you have no attack queued and if you roll a 1d10 greater than
    // the enemy's 1d10 + their dominance.
    pub fn can_move(
        &self,
        target_attributes: &Attributes,
        queued_attack: &Option<&QueuedAttack>,
    ) -> bool {
        if queued_attack.is_some() {
            return false;
        }

        let attacker_roller = Roller::new("1d10").unwrap();
        let attacker_roll = attacker_roller.roll().unwrap();
        let attacker_roll = attacker_roll.as_single().unwrap().get_total();

        let target_roller = Roller::new("1d10").unwrap();
        let target_roll = target_roller.roll().unwrap();
        let target_roll = target_roll.as_single().unwrap().get_total();

        attacker_roll > target_roll + target_attributes.dominance as i64
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
