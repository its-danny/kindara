use std::fmt::{self, Display, Formatter};

use bevy::{prelude::*, utils::HashMap};
use serde::Deserialize;

use crate::data::resources::DamageKind;
use crate::{data::resources::Stat, input::events::ParsedCommand};

use crate::values::{
    ATTACK_SPEED_CAP, ATTACK_SPEED_FACTOR, AUTO_ATTACK_LEVEL_CONTRIBUTION,
    AUTO_ATTACK_SPEED_FACTOR, AUTO_ATTACK_STAT_CONTRIBUTION, BASE_ATTACK_SPEED,
    BASE_AUTO_ATTACK_DAMAGE, BASE_BLOCK_CHANCE, BASE_BLOCK_RATE, BASE_CRIT_DAMAGE_MULTIPLIER,
    BASE_CRIT_STRIKE_CHANCE, BASE_DODGE_CHANCE, BASE_DODGE_RATE, BASE_FLEE_CHANCE, BASE_HEALTH,
    BASE_HEALTH_REGEN, BASE_VIGOR, BASE_VIGOR_REGEN, BLOCK_CHANCE_CAP,
    BLOCK_CHANCE_STAT_CONTRIBUTION, BLOCK_CHANCE_STRENGTH_CONTRIBUTION, BLOCK_RATE_CAP,
    BLOCK_RATE_STAT_CONTRIBUTION, BLOCK_RATE_STRENGTH_CONTRIBUTION, CRIT_DAMAGE_STAT_CONTRIBUTION,
    CRIT_STRIKE_CHANCE_CAP, CRIT_STRIKE_STAT_CONTRIBUTION, DODGE_CHANCE_CAP,
    DODGE_CHANCE_DEXTERITY_CONTRIBUTION, DODGE_CHANCE_STAT_CONTRIBUTION, DODGE_RATE_CAP,
    DODGE_RATE_DEXTERITY_CONTRIBUTION, DODGE_RATE_STAT_CONTRIBUTION,
    FLEE_CHANCE_DOMINANCE_CONTRIBUTION, FLEE_CHANCE_FLEET_CONTRIBUTION,
    HEALTH_REGEN_STAT_CONTRIBUTION, HEALTH_REGEN_TICK, MAX_HEALTH_LEVEL_CONTRIBUTION,
    MAX_HEALTH_STAT_CONTRIBUTION, MAX_VIGOR_LEVEL_CONTRIBUTION, MAX_VIGOR_STAT_CONTRIBUTION,
    RESISTANCE_CAP, RESISTANCE_FACTOR, VIGOR_REGEN_STAT_CONTRIBUTION, VIGOR_REGEN_TICK,
};

#[derive(Component, Deserialize, Debug, Default, Reflect, Clone)]
pub struct Stats {
    pub level: u32,
    #[reflect(default)]
    pub attributes: Attributes,
    #[reflect(default)]
    pub status: Status,
    #[reflect(default)]
    pub defense: Defense,
    #[reflect(default)]
    pub offense: Offense,
    #[reflect(default)]
    pub resistance: Resistance,
}

#[derive(Default, Deserialize, Debug, Reflect, Clone)]
pub struct Attributes {
    #[reflect(default)]
    pub vitality: u32,
    #[reflect(default)]
    pub stamina: u32,
    #[reflect(default)]
    pub strength: u32,
    #[reflect(default)]
    pub dexterity: u32,
    #[reflect(default)]
    pub intelligence: u32,
}

#[derive(Default, Deserialize, Debug, Reflect, Clone)]
pub struct Status {
    #[reflect(default)]
    pub health: u32,
    #[reflect(default)]
    pub vigor: u32,
    #[reflect(default)]
    pub vigor_regen: u32,
}

#[derive(Default, Deserialize, Debug, Reflect, Clone)]
pub struct Defense {
    #[reflect(default)]
    pub dodge_chance: f32,
    #[reflect(default)]
    pub dodge_rate: f32,
    #[reflect(default)]
    pub block_chance: f32,
    #[reflect(default)]
    pub block_rate: f32,
    #[reflect(default)]
    pub fleet: u32,
}

#[derive(Default, Deserialize, Debug, Reflect, Clone)]
pub struct Offense {
    #[reflect(default)]
    pub attack_speed: u32,
    #[reflect(default)]
    pub dominance: u32,
    #[reflect(default)]
    pub crit_strike_chance: f32,
    #[reflect(default)]
    pub crit_strike_damage: f32,
}

#[derive(Default, Deserialize, Debug, Reflect, Clone)]
pub struct Resistance(pub HashMap<String, u32>);

impl Stats {
    pub fn max_health(&self) -> u32 {
        let mut max_health =
            BASE_HEALTH + (self.attributes.vitality as f32 * MAX_HEALTH_STAT_CONTRIBUTION);

        for _ in 1..self.level {
            max_health += max_health * MAX_HEALTH_LEVEL_CONTRIBUTION;
        }

        f32::floor(max_health) as u32
    }

    pub fn health_per_second(&self) -> u32 {
        BASE_HEALTH_REGEN
            + ((self.attributes.vitality as f32 * HEALTH_REGEN_STAT_CONTRIBUTION).floor() as u32)
    }

    pub fn max_vigor(&self) -> u32 {
        let mut max_vigor =
            BASE_VIGOR + (self.attributes.stamina as f32 * MAX_VIGOR_STAT_CONTRIBUTION);

        for _ in 1..self.level {
            max_vigor += max_vigor * MAX_VIGOR_LEVEL_CONTRIBUTION;
        }

        f32::floor(max_vigor) as u32
    }

    pub fn vigor_per_second(&self) -> u32 {
        BASE_VIGOR_REGEN
            + ((self.attributes.stamina as f32 * VIGOR_REGEN_STAT_CONTRIBUTION).floor() as u32)
    }

    pub fn attack_speed(&self) -> f32 {
        f32::max(
            BASE_ATTACK_SPEED / (1.0 + (self.offense.attack_speed as f32 * ATTACK_SPEED_FACTOR)),
            ATTACK_SPEED_CAP,
        )
    }

    pub fn auto_attack_speed(&self) -> f32 {
        self.attack_speed() * AUTO_ATTACK_SPEED_FACTOR
    }

    pub fn auto_attack_damage(&self) -> u32 {
        let highest_stat = self
            .attributes
            .strength
            .max(self.attributes.dexterity)
            .max(self.attributes.intelligence);

        (BASE_AUTO_ATTACK_DAMAGE + (self.level * AUTO_ATTACK_LEVEL_CONTRIBUTION))
            + (highest_stat as f32 * AUTO_ATTACK_STAT_CONTRIBUTION) as u32
    }

    pub fn dodge_chance(&self, manual_dodge: bool, difficulty: &f32) -> f32 {
        if manual_dodge {
            1.0 - difficulty
        } else {
            f32::min(
                BASE_DODGE_CHANCE
                    + (self.attributes.dexterity as f32 * DODGE_CHANCE_DEXTERITY_CONTRIBUTION)
                    + (self.defense.dodge_chance * DODGE_CHANCE_STAT_CONTRIBUTION)
                    - difficulty,
                DODGE_CHANCE_CAP,
            )
        }
    }

    pub fn dodge_cooldown(&self) -> f32 {
        f32::max(
            BASE_DODGE_RATE
                - (self.attributes.dexterity as f32 * DODGE_RATE_DEXTERITY_CONTRIBUTION)
                - (self.defense.dodge_rate * DODGE_RATE_STAT_CONTRIBUTION),
            DODGE_RATE_CAP,
        )
    }

    pub fn block_chance(&self, manual_block: bool, difficulty: &f32) -> f32 {
        if manual_block {
            1.0 - difficulty
        } else {
            f32::min(
                BASE_BLOCK_CHANCE
                    + (self.attributes.strength as f32 * BLOCK_CHANCE_STRENGTH_CONTRIBUTION)
                    + (self.defense.block_chance * BLOCK_CHANCE_STAT_CONTRIBUTION)
                    - difficulty,
                BLOCK_CHANCE_CAP,
            )
        }
    }

    pub fn block_cooldown(&self) -> f32 {
        f32::max(
            BASE_BLOCK_RATE
                - (self.attributes.strength as f32 * BLOCK_RATE_STRENGTH_CONTRIBUTION)
                - (self.defense.block_rate * BLOCK_RATE_STAT_CONTRIBUTION),
            BLOCK_RATE_CAP,
        )
    }

    pub fn flee_chance(&self, dominance: &f32) -> f32 {
        BASE_FLEE_CHANCE - (dominance * FLEE_CHANCE_DOMINANCE_CONTRIBUTION)
            + (self.defense.fleet as f32 * FLEE_CHANCE_FLEET_CONTRIBUTION)
    }

    pub fn critical_strike_chance(&self) -> f32 {
        f32::min(
            BASE_CRIT_STRIKE_CHANCE
                + (self.offense.crit_strike_chance * CRIT_STRIKE_STAT_CONTRIBUTION),
            CRIT_STRIKE_CHANCE_CAP,
        )
    }

    pub fn critical_strike_damage(&self) -> f32 {
        BASE_CRIT_DAMAGE_MULTIPLIER
            + (self.offense.crit_strike_damage * CRIT_DAMAGE_STAT_CONTRIBUTION)
    }

    pub fn resisted(&self, damage_kind: &DamageKind) -> u32 {
        let resistances = self.resistance.0.iter().fold(0, |acc, (key, value)| {
            if damage_kind.resistances.contains(key) {
                acc + value
            } else {
                acc
            }
        });

        f32::floor(f32::min(
            resistances as f32 * RESISTANCE_FACTOR,
            RESISTANCE_CAP,
        )) as u32
    }
}

#[derive(Component, Reflect, Default, Clone)]
pub struct Cooldowns(pub HashMap<String, (Entity, Timer)>);

#[derive(Component, Reflect, Default, Clone)]
pub struct Conditions(pub HashMap<String, Option<Timer>>);

#[derive(Component, Reflect, Default, Clone)]
pub struct Modifiers(pub HashMap<String, (Stat, f32)>);

impl Modifiers {
    pub fn sum_stat(&self, stat: &Stat) -> f32 {
        self.0
            .values()
            .filter_map(|(s, v)| if s == stat { Some(*v) } else { None })
            .sum()
    }
}

#[derive(Component, Clone)]
pub struct CombatState {
    pub target: Entity,
    pub distance: Distance,
    pub approach: Approach,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
pub enum Distance {
    Near,
    Far,
    Either,
}

impl Display for Distance {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Distance::Near => write!(f, "near"),
            Distance::Far => write!(f, "far"),
            Distance::Either => write!(f, "either"),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Approach {
    Front,
    Rear,
}

impl Display for Approach {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Approach::Front => write!(f, "front"),
            Approach::Rear => write!(f, "rear"),
        }
    }
}

#[derive(Component)]
pub struct AutoAttackTimer(pub Timer);

#[derive(Component)]
pub struct AttackTimer(pub Timer);

#[derive(Component)]
pub struct QueuedAttack(pub ParsedCommand);

#[derive(Component)]
pub struct ManualDodge(pub Timer);

#[derive(Component)]
pub struct DodgeCooldown(pub Timer);

#[derive(Component)]
pub struct ManualBlock(pub Timer);

#[derive(Component)]
pub struct BlockCooldown(pub Timer);

#[derive(Component)]
pub struct FleeTimer(pub Timer);

#[derive(Component, Reflect, Clone)]
pub struct HealthRegenTimer(pub Timer);

impl Default for HealthRegenTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(HEALTH_REGEN_TICK, TimerMode::Repeating))
    }
}

#[derive(Component, Reflect, Clone)]
pub struct VigorRegenTimer(pub Timer);

impl Default for VigorRegenTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(VIGOR_REGEN_TICK, TimerMode::Repeating))
    }
}
