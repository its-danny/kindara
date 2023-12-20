use std::{
    cmp::max,
    fmt::{self, Display, Formatter},
};

use bevy::prelude::*;
use caith::Roller;
use serde::Deserialize;

use crate::{
    input::events::ParsedCommand,
    skills::{
        components::Bleeding,
        resources::{Action, DamageType, RelevantStat, Skill, StatusEffect},
    },
};

#[derive(Component, Default, Reflect, Clone)]
pub struct Stats {
    pub level: u32,
    // --- Attributes
    /// Determines max health and health regen.
    pub vitality: u32,
    /// Determines max potential and potential regen.
    pub proficiency: u32,
    /// Increases damage of relevant skills and block chance.
    pub strength: u32,
    /// Increases damage of relevant skills and dodge chance.
    pub dexterity: u32,
    /// Increases damage of relevant skills.
    pub intelligence: u32,
    // --- State
    /// Current health.
    pub health: u32,
    /// Current potential.
    pub potential: u32,
    /// Potential regen per second.
    pub potential_regen: u32,
    // --- Resistance
    /// Resistance to physical damage.
    pub armor: u32,
    // --- Defense
    /// Chance to dodge an attack.
    pub dodge_chance: u32,
    /// Chance to block an attack.
    pub block_chance: u32,
    // --- Offense
    /// Attack speed in seconds.
    pub speed: u32,
    /// Decreases the chance of target fleeing.
    pub dominance: u32,
    /// How likely you are to hit a crit.
    pub crit_strike_chance: u32,
    /// How much damage is done from a crit.
    pub crit_strike_damage: u32,
}

static BASE_POTENTIAL_REGEN: u32 = 1;
static BASE_CRIT_THRESHOLD: u32 = 20;
static BASE_CRIT_STRIKE_DAMAGE: u32 = 10;
static CRIT_THRESHOLD_CAP: u32 = 5;

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

    pub fn get_relevant_stat(&self, stat: &RelevantStat) -> u32 {
        match stat {
            RelevantStat::Strength => self.strength,
            RelevantStat::Dexterity => self.dexterity,
            RelevantStat::Intelligence => self.intelligence,
        }
    }

    pub fn hit(&self, skill: &Skill, attacker_stats: &Stats) -> Result<(), HitError> {
        let quality = roll_as_single("2d10") as u32 + attacker_stats.get_relevant_stat(&skill.stat);

        let dodge = roll_as_single("2d10") as u32 + self.dexterity + self.dodge_chance;
        let block = roll_as_single("2d10") as u32 + self.strength + self.block_chance;

        if quality <= dodge {
            Err(HitError::Dodged)
        } else if quality <= block {
            Err(HitError::Blocked)
        } else {
            Ok(())
        }
    }

    pub fn deal_damage(
        &mut self,
        roll: &str,
        attacker_stats: &Stats,
        relevant_stat: Option<&RelevantStat>,
        damage_type: Option<&DamageType>,
        difficulty: &u32,
    ) -> u32 {
        // Set difficulty.

        let difficulty = (*difficulty as f32 + (attacker_stats.level as f32 * 0.5)).floor() as u32;

        // Roll for base damage and add relevant stat modifier.

        let mut damage = roll_as_single(roll) as u32;
        damage += relevant_stat.map_or(0, |stat| attacker_stats.get_relevant_stat(stat));

        // Check for crit and add crit damage.

        let crit_roll = roll_as_single("2d10") as u32;
        let crit_threshold = BASE_CRIT_THRESHOLD.saturating_sub(attacker_stats.crit_strike_chance);
        let crit_threshold = std::cmp::max(crit_threshold, CRIT_THRESHOLD_CAP);

        damage += if crit_roll >= crit_threshold {
            let crit_dmg_roll = roll_as_single("2d10") as u32;

            max(crit_dmg_roll, BASE_CRIT_STRIKE_DAMAGE) + self.crit_strike_damage
        } else {
            0
        };

        // Apply resistance.

        let resistance = roll_as_single("2d10") as u32;

        let res_modifier = if let Some(damage_type) = damage_type {
            match damage_type {
                DamageType::Physical => self.armor,
            }
        } else {
            0
        };

        let resistance = resistance + res_modifier;
        let excess = resistance.saturating_sub(difficulty);

        let mitigated = match excess {
            0..=4 => 0.25,
            5..=8 => 0.5,
            9..=12 => 0.75,
            _ => 1.0,
        };

        damage = (damage as f32 * (1.0 - mitigated)) as u32;

        // Apply damage.

        self.health = self.health.saturating_sub(damage);

        damage
    }
}

pub enum HitError {
    Dodged,
    Blocked,
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

        target_stats.hit(skill, attacker_stats)?;

        let damage = self.apply_actions(bevy, skill, &attacker, attacker_stats, target_stats);

        Ok(damage)
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
                    let damage = target_stats.deal_damage(
                        roll,
                        attacker_stats,
                        Some(&skill.stat),
                        Some(&skill.damage_type),
                        &skill.difficulty,
                    );

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

    // You can move if you have no attack queued and if you roll a 2d10 greater than
    // the enemy's 2d10 + their dominance.
    pub fn can_move(&self, target_stats: &Stats, queued_attack: &Option<&QueuedAttack>) -> bool {
        if queued_attack.is_some() {
            return false;
        }

        let attacker_roll = roll_as_single("2d10");
        let target_roll = roll_as_single("2d10");

        attacker_roll > target_roll + target_stats.dominance as i64
    }
}

fn roll_as_single(roll: &str) -> i64 {
    let roller = Roller::new(roll).unwrap();
    let roll = roller.roll().unwrap();
    roll.as_single().unwrap().get_total()
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

/// Added to an entity when it has attacked to prevent acting faster
/// than their attack speed. The timer is handled via the `update_attack_timer` system.
#[derive(Component)]
pub struct HasAttacked {
    pub timer: Timer,
}

#[derive(Component)]
pub struct QueuedAttack(pub ParsedCommand);
