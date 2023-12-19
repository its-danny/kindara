use std::fmt::{self, Display, Formatter};

use bevy::prelude::*;
use caith::Roller;
use serde::Deserialize;

use crate::{
    input::events::ParsedCommand,
    skills::{
        components::Bleeding,
        resources::{Action, RelevantStat, Skill, StatusEffect},
    },
};

#[derive(Component, Default, Reflect, Clone)]
pub struct Stats {
    // --- Attributes
    /// Determines base health, max health, and health regeneration amount.
    pub vitality: u32,
    /// Determines max potential, and potential regeneration amount.
    pub proficiency: u32,
    /// Determines attack speed.
    pub speed: u32,
    /// Modifier for brute force attacks.
    pub strength: u32,
    /// Modifier for finesse attacks.
    pub dexterity: u32,
    /// Modifier for magic attacks.
    pub intelligence: u32,
    // --- State
    pub health: u32,
    pub potential: u32,
    // How much potential is regenerated per second.
    pub potential_regen: u32,
    // --- Offense
    /// How likely an entity is to flee from you.
    pub dominance: u32,
}

static BASE_POTENTIAL_REGEN: u32 = 1;

impl Stats {
    pub fn max_health(&self) -> u32 {
        self.vitality * 10
    }

    pub fn max_potential(&self) -> u32 {
        self.proficiency * 10
    }

    pub fn potential_per_second(&self) -> u32 {
        BASE_POTENTIAL_REGEN + self.potential_regen
    }

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
        attacker_stats: &Stats,
        target_stats: &mut Stats,
    ) -> Result<u32, HitError> {
        bevy.entity(attacker).insert(HasAttacked {
            timer: Timer::from_seconds(attacker_stats.speed as f32, TimerMode::Once),
        });

        self.roll_hit()?;

        let damage = self.apply_actions(bevy, skill, &attacker, attacker_stats, target_stats);

        Ok(damage)
    }

    fn roll_hit(&self) -> Result<(), HitError> {
        let quality = self.roll_as_single("2d10");
        let dodge = self.roll_as_single("1d10");

        if quality < dodge {
            Err(HitError::Missed)
        } else {
            Ok(())
        }
    }

    fn apply_actions(
        &self,
        bevy: &mut Commands,
        skill: &Skill,
        attacker: &Entity,
        attacker_stats: &Stats,
        target_stats: &mut Stats,
    ) -> u32 {
        let mut damage_done = 0_u32;

        for action in &skill.actions {
            match action {
                Action::ApplyDamage(roll) => {
                    let mut damage = self.roll_as_single(roll) as u32;

                    damage += match &skill.stat {
                        RelevantStat::Strength => attacker_stats.strength,
                        RelevantStat::Dexterity => attacker_stats.dexterity,
                        RelevantStat::Intelligence => attacker_stats.intelligence,
                    };

                    target_stats.apply_damage(damage);

                    damage_done += damage;
                }
                Action::ApplyStatus(status, roll, tick, duration) => match status {
                    StatusEffect::Bleeding => {
                        bevy.entity(self.target).insert(Bleeding {
                            source: *attacker,
                            tick: Timer::from_seconds(*tick as f32, TimerMode::Repeating),
                            duration: Timer::from_seconds(*duration as f32, TimerMode::Once),
                            roll: roll.clone(),
                        });
                    }
                },
            }
        }

        damage_done
    }

    // You can move if you have no attack queued and if you roll a 1d10 greater than
    // the enemy's 1d10 + their dominance.
    pub fn can_move(&self, target_stats: &Stats, queued_attack: &Option<&QueuedAttack>) -> bool {
        if queued_attack.is_some() {
            return false;
        }

        let attacker_roll = self.roll_as_single("2d10");
        let target_roll = self.roll_as_single("2d10");

        attacker_roll > target_roll + target_stats.dominance as i64
    }

    fn roll_as_single(&self, roll: &str) -> i64 {
        let roller = Roller::new(roll).unwrap();
        let roll = roller.roll().unwrap();
        roll.as_single().unwrap().get_total()
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
