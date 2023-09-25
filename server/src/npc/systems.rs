use bevy::prelude::*;
use bevy_nest::prelude::*;
use bevy_proto::prelude::*;
use rand::{thread_rng, Rng};

use crate::{
    combat::{
        components::{Attributes, HasAttacked, InCombat, State},
        rolls::{roll_for_attack_quality, roll_for_dodge, roll_total},
    },
    player::{
        components::{Client, Online},
        events::Prompt,
    },
    skills::resources::{Action, RelevantStat, Skills},
    spatial::components::Tile,
    value_or_continue,
    visual::components::Depiction,
};

use super::components::{EnemySpawner, Npc, SpawnTimer};

pub fn on_enter_combat(
    mut bevy: Commands,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut State), With<Online>>,
    mut prompts: EventWriter<Prompt>,
    npcs: Query<(Entity, &Npc, &Depiction, &Attributes, &InCombat), Added<InCombat>>,
    skills: Res<Skills>,
) {
    for (entity, npc, depiction, attributes, in_combat) in npcs.iter() {
        let (client, mut state) = value_or_continue!(players.get_mut(in_combat.0).ok());

        let mut rng = thread_rng();
        let index = rng.gen_range(0..npc.skills.len());
        let skill = value_or_continue!(skills.0.get(&npc.skills[index]));

        let quality = roll_for_attack_quality();
        let dodge = roll_for_dodge();

        if quality < dodge {
            outbox.send_text(
                client.id,
                format!("{} attacks you but misses.", depiction.name),
            );

            continue;
        }

        for action in &skill.actions {
            match action {
                Action::ApplyDamage(roll) => {
                    let mut damage = roll_total(roll) as u32;

                    damage += match &skill.stat {
                        RelevantStat::Strength => attributes.strength,
                        RelevantStat::Dexterity => attributes.dexterity,
                        RelevantStat::Intelligence => attributes.intelligence,
                    };

                    state.apply_damage(damage);
                }
            }
        }

        bevy.entity(entity).insert(HasAttacked {
            timer: Timer::from_seconds(attributes.speed as f32, TimerMode::Once),
        });

        outbox.send_text(
            client.id,
            format!(
                "{} attacks you. Your health is now {}.",
                depiction.name, state.health
            ),
        );

        prompts.send(Prompt::new(client.id));
    }
}

pub fn attack_when_able(
    mut bevy: Commands,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut State), With<Online>>,
    mut prompts: EventWriter<Prompt>,
    mut ready: RemovedComponents<HasAttacked>,
    npcs: Query<(Entity, &Npc, &Depiction, &Attributes, &InCombat)>,
    skills: Res<Skills>,
) {
    for entity in ready.iter() {
        let (entity, npc, depiction, attributes, in_combat) =
            value_or_continue!(npcs.get(entity).ok());
        let (client, mut state) = value_or_continue!(players.get_mut(in_combat.0).ok());

        let mut rng = thread_rng();
        let index = rng.gen_range(0..npc.skills.len());
        let skill = value_or_continue!(skills.0.get(&npc.skills[index]));

        let quality = roll_for_attack_quality();
        let dodge = roll_for_dodge();

        if quality < dodge {
            outbox.send_text(
                client.id,
                format!("{} attacks you but misses.", depiction.name),
            );

            continue;
        }

        for action in &skill.actions {
            match action {
                Action::ApplyDamage(roll) => {
                    let mut damage = roll_total(roll) as u32;

                    damage += match &skill.stat {
                        RelevantStat::Strength => attributes.strength,
                        RelevantStat::Dexterity => attributes.dexterity,
                        RelevantStat::Intelligence => attributes.intelligence,
                    };

                    state.apply_damage(damage);
                }
            }
        }

        bevy.entity(entity).insert(HasAttacked {
            timer: Timer::from_seconds(attributes.speed as f32, TimerMode::Once),
        });

        outbox.send_text(
            client.id,
            format!(
                "{} attacks you. Your health is now {}.",
                depiction.name, state.health
            ),
        );

        prompts.send(Prompt::new(client.id));
    }
}

pub fn handle_enemy_spawner(
    mut bevy: Commands,
    mut proto: ProtoCommands,
    mut spawners: Query<(Entity, &Parent, &mut EnemySpawner, Option<&mut SpawnTimer>)>,
    prototypes: Prototypes,
    tiles: Query<Entity, With<Tile>>,
    time: Res<Time>,
) {
    for (entity, tile, mut spawner, timer) in spawners.iter_mut() {
        if !prototypes.is_ready(&spawner.enemies.0) {
            continue;
        }

        let tile = value_or_continue!(tiles.get(tile.get()).ok());

        if let Some(mut timer) = timer {
            if timer.0.tick(time.delta()).just_finished() {
                let mut rng = thread_rng();
                let range = spawner.enemies.1..=spawner.enemies.2;
                let amount = rng.gen_range(range);

                for _ in 0..amount {
                    let enemy = proto.spawn(&spawner.enemies.0);
                    bevy.entity(enemy.id()).set_parent(tile);

                    spawner.spawned.push(enemy.id());
                }

                bevy.entity(entity).remove::<SpawnTimer>();
            }
        } else if spawner.spawned.is_empty() {
            bevy.entity(entity).insert(SpawnTimer(Timer::from_seconds(
                spawner.delay,
                TimerMode::Once,
            )));
        }
    }
}
