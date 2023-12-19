use std::cmp::min;

use bevy::prelude::*;

use crate::combat::components::Stats;

use super::components::{Cooldowns, PotentialRegenTimer};

pub fn potential_regen(time: Res<Time>, mut timers: Query<(&mut Stats, &mut PotentialRegenTimer)>) {
    for (mut stats, mut timer) in timers.iter_mut() {
        if timer.0.tick(time.delta()).just_finished() {
            stats.potential = min(
                stats.potential + stats.potential_per_second(),
                stats.max_potential(),
            );
        }
    }
}

pub fn update_cooldowns(time: Res<Time>, mut cooldowns: Query<&mut Cooldowns>) {
    for mut cooldowns in cooldowns.iter_mut() {
        let finished: Vec<String> = cooldowns
            .0
            .iter_mut()
            .filter_map(|(skill, timer)| {
                if timer.tick(time.delta()).just_finished() {
                    Some(skill.clone()) // Clone the key here
                } else {
                    None
                }
            })
            .collect();

        for skill in finished {
            cooldowns.0.remove(&skill);
        }
    }
}
