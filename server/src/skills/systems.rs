use std::cmp::min;

use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;

use crate::{
    combat::components::{Cooldowns, PotentialRegenTimer, Stats},
    data::resources::DamageType,
    paint,
    player::components::{Character, Client, Online},
    visual::components::Depiction,
};

use super::components::Bleeding;

pub fn potential_regen(time: Res<Time>, mut timers: Query<(&mut Stats, &mut PotentialRegenTimer)>) {
    for (mut stats, mut timer) in timers.iter_mut() {
        if timer.0.tick(time.delta()).just_finished() {
            stats.state.potential = min(
                stats.state.potential + stats.potential_per_second(),
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
                    Some(skill.clone())
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

#[sysfail(log)]
pub fn update_bleeding(
    mut bevy: Commands,
    time: Res<Time>,
    mut outbox: EventWriter<Outbox>,
    mut targets: Query<(
        Entity,
        &mut Bleeding,
        Option<&Depiction>,
        Option<&Character>,
    )>,
    players: Query<&Client, With<Online>>,
    mut stats: Query<&mut Stats>,
) -> Result<(), anyhow::Error> {
    for (target, mut bleeding, depiction, character) in targets.iter_mut() {
        if bleeding.duration.tick(time.delta()).just_finished() {
            bevy.entity(target).remove::<Bleeding>();

            if let (Ok(client), Some(depiction)) = (players.get(bleeding.source), depiction) {
                outbox.send_text(
                    client.id,
                    format!("{} is no longer bleeding.", depiction.name),
                );
            }

            if let (Ok(client), Some(_)) = (players.get(target), character) {
                outbox.send_text(client.id, "You are no longer bleeding.");
            }
        } else if bleeding.tick.tick(time.delta()).just_finished() {
            let attacker_stats = stats.get(bleeding.source)?.clone();
            let mut target_stats = stats.get_mut(target)?;

            let damage = target_stats.deal_damage(
                &bleeding.roll,
                &attacker_stats,
                None,
                Some(&DamageType::Physical),
                &10,
            );

            if let (Ok(client), Some(depiction)) = (players.get(bleeding.source), depiction) {
                outbox.send_text(
                    client.id,
                    paint!(
                        "{} takes <fg.red>{}</> damage from bleeding.",
                        depiction.name,
                        damage
                    ),
                );
            }

            if let (Ok(client), Some(_)) = (players.get(target), character) {
                outbox.send_text(
                    client.id,
                    paint!("You take <fg.red>{}</> damage from bleeding.", damage),
                );
            }
        }
    }

    Ok(())
}
