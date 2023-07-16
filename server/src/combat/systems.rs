use bevy::prelude::*;

use super::components::HasAttacked;

pub fn update_attack_timer(
    mut bevy: Commands,
    mut timers: Query<(Entity, &mut HasAttacked)>,
    time: Res<Time>,
) {
    for (entity, mut has_attacked) in timers.iter_mut() {
        has_attacked.timer.tick(time.delta());

        if has_attacked.timer.finished() {
            bevy.entity(entity).remove::<HasAttacked>();
        }
    }
}
