use std::cmp::min;

use bevy::prelude::*;

use crate::combat::components::Stats;

use super::components::PotentialRegenTimer;

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
