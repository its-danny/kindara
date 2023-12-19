use std::cmp::min;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use caith::Roller;

use crate::{
    combat::components::Stats,
    paint,
    player::components::{Client, Online},
    visual::components::Depiction,
};

use super::components::{Bleeding, Cooldowns, PotentialRegenTimer};

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

pub fn update_bleeding(
    mut bevy: Commands,
    time: Res<Time>,
    mut targets: Query<(Entity, &Depiction, &mut Stats, &mut Bleeding)>,
    players: Query<(Entity, &Client), With<Online>>,
    mut outbox: EventWriter<Outbox>,
) {
    for (target, depiction, mut stats, mut bleeding) in targets.iter_mut() {
        if bleeding.length.tick(time.delta()).just_finished() {
            bevy.entity(target).remove::<Bleeding>();
        } else if bleeding.tick.tick(time.delta()).just_finished() {
            let roller = Roller::new(&bleeding.roll).unwrap();
            let roll = roller.roll().unwrap();
            let damage = roll.as_single().unwrap().get_total() as u32;

            stats.health -= damage;

            if let Ok((_, client)) = players.get(bleeding.source) {
                outbox.send_text(
                    client.id,
                    paint!(
                        "{} takes <fg.red>{}</> damage from bleeding.",
                        depiction.name,
                        damage
                    ),
                );
            }
        }
    }
}
