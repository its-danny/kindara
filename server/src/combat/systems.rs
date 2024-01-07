use anyhow::Context;
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use inflector::Inflector;
use mlua::prelude::*;

use crate::{
    data::{
        self,
        resources::{DamageKinds, Masteries, Skill, Skills, Stat},
    },
    input::events::{Command, ParsedCommand, ProxyCommand},
    lua::{
        context::{ExecutionContext, ExecutionKind},
        events::{ApplyDamageResponse, ExecutionEvent, ExecutionPhase},
    },
    npc::components::Hostile,
    paint,
    player::{
        components::{Character, Client, Online},
        events::Prompt,
    },
    spatial::{
        components::{DeathSpawn, Tile},
        events::{MovementEvent, MovementEventKind},
    },
    values::FLEE_COOLDOWN,
    visual::components::Depiction,
};

use super::{
    components::{
        AttackTimer, AutoAttackTimer, BlockCooldown, CombatState, Conditions, Cooldowns, Distance,
        DodgeCooldown, FleeTimer, HealthRegenTimer, ManualBlock, ManualDodge, Modifiers,
        QueuedAttack, Stats, VigorRegenTimer,
    },
    events::{CombatEvent, CombatEventKind, CombatEventTrigger, CombatLogKind, WithCallback},
};

#[derive(SystemParam)]
pub struct CharacterOrHostile<'w, 's> {
    characters: Query<'w, 's, &'static Character>,
    hostiles: Query<'w, 's, &'static Hostile>,
    masteries: Res<'w, Masteries>,
    skills: Res<'w, Skills>,
}

impl<'w, 's> CharacterOrHostile<'w, 's> {
    fn get_auto_attack(&self, entity: Entity) -> Result<Skill, anyhow::Error> {
        let skill_id = if let Ok(character) = self.characters.get(entity) {
            Ok(self
                .masteries
                .0
                .get(&character.mastery)
                .with_context(|| format!("Mastery not found: {}", character.mastery))?
                .auto_attack
                .clone())
        } else if let Ok(hostile) = self.hostiles.get(entity) {
            Ok(hostile.auto_attack.clone())
        } else {
            Err(anyhow::anyhow!("Failed to find character or hostile"))
        };

        let skill_id = skill_id?;

        let skill = self
            .skills
            .0
            .get(&skill_id)
            .with_context(|| format!("Failed to find skill: {}", skill_id))?;

        Ok(skill.clone())
    }
}

pub fn start_auto_attacks(
    mut bevy: Commands,
    fighters: Query<(Entity, &Stats), Added<CombatState>>,
) {
    for (entity, stats) in fighters.iter() {
        bevy.entity(entity)
            .insert(AutoAttackTimer(Timer::from_seconds(
                stats.auto_attack_speed(),
                TimerMode::Repeating,
            )));
    }
}

#[sysfail(log)]
pub fn handle_auto_attack(
    time: Res<Time>,
    mut fighters: Query<(Entity, &mut AutoAttackTimer)>,
    mut events: EventWriter<CombatEvent>,
    character_or_hostile: CharacterOrHostile,
) -> Result<(), anyhow::Error> {
    for (entity, mut timer) in fighters.iter_mut() {
        let skill = character_or_hostile.get_auto_attack(entity)?;

        if timer.0.tick(time.delta()).just_finished() {
            events.send(CombatEvent {
                source: entity,
                trigger: CombatEventTrigger::Skill(skill),
                kind: CombatEventKind::Attack,
            });
        }
    }

    Ok(())
}

pub fn stop_auto_attacks(
    mut bevy: Commands,
    mut ready: RemovedComponents<CombatState>,
    fighters: Query<Entity>,
) {
    for entity in ready.iter() {
        if let Ok(entity) = fighters.get(entity) {
            bevy.entity(entity).remove::<AutoAttackTimer>();
        }
    }
}

#[sysfail(log)]
pub fn on_combat_event_attack(
    mut bevy: Commands,
    mut events: ParamSet<(EventReader<CombatEvent>, EventWriter<CombatEvent>)>,
    mut fighters: Query<(&mut Stats, &mut Cooldowns)>,
) -> Result<(), anyhow::Error> {
    let mut events_to_send: Vec<CombatEvent> = vec![];

    for event in events.p0().iter() {
        if let CombatEventKind::Attack = &event.kind {
            if let CombatEventTrigger::Skill(skill) = &event.trigger {
                let (mut stats, mut cooldowns) = fighters.get_mut(event.source)?;

                bevy.entity(event.source)
                    .insert(AttackTimer(Timer::from_seconds(
                        stats.attack_speed(),
                        TimerMode::Once,
                    )));

                stats.status.vigor = stats.status.vigor.saturating_sub(skill.cost);

                if skill.cooldown > 0 {
                    cooldowns.0.insert(
                        skill.id.clone(),
                        (
                            event.source,
                            Timer::from_seconds(skill.cooldown as f32, TimerMode::Once),
                        ),
                    );
                }

                events_to_send.push(CombatEvent {
                    source: event.source,
                    trigger: event.trigger.clone(),
                    kind: CombatEventKind::ExecuteScripts(ExecutionPhase::OnUse),
                });

                events_to_send.push(CombatEvent {
                    source: event.source,
                    trigger: event.trigger.clone(),
                    kind: CombatEventKind::AttemptHit,
                });
            }
        }
    }

    for event in events_to_send {
        events.p1().send(event);
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_attempt_hit(
    mut combat_events: ParamSet<(EventReader<CombatEvent>, EventWriter<CombatEvent>)>,
    fighters: Query<&CombatState>,
) -> Result<(), anyhow::Error> {
    let mut events_to_send: Vec<CombatEvent> = vec![];

    for event in combat_events.p0().iter() {
        if let CombatEventKind::AttemptHit = &event.kind {
            if let CombatEventTrigger::Skill(skill) = &event.trigger {
                let source_combat_state = fighters.get(event.source)?;

                if skill.distance != Distance::Either
                    && source_combat_state.distance != skill.distance
                {
                    events_to_send.push(CombatEvent {
                        source: event.source,
                        trigger: event.trigger.clone(),
                        kind: CombatEventKind::ExecuteScripts(ExecutionPhase::OnMiss),
                    });
                } else {
                    events_to_send.push(CombatEvent {
                        source: event.source,
                        trigger: event.trigger.clone(),
                        kind: CombatEventKind::AttemptDodge,
                    });
                }
            }
        }
    }

    for event in events_to_send {
        combat_events.p1().send(event);
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_attempt_dodge(
    mut bevy: Commands,
    mut combat_events: ParamSet<(EventReader<CombatEvent>, EventWriter<CombatEvent>)>,
    fighters: Query<(
        &CombatState,
        &Stats,
        Option<&ManualDodge>,
        Option<&DodgeCooldown>,
    )>,
) -> Result<(), anyhow::Error> {
    let mut events_to_send: Vec<CombatEvent> = vec![];

    for event in combat_events.p0().iter() {
        if let CombatEventKind::AttemptDodge = &event.kind {
            let (source_combat_state, _, _, _) = fighters.get(event.source)?;
            let (_, target_stats, target_manual_dodge, target_dodge_timer) =
                fighters.get(source_combat_state.target)?;

            if target_dodge_timer.is_some() {
                continue;
            }

            let skill = match &event.trigger {
                CombatEventTrigger::Skill(skill) => Some(skill),
                _ => None,
            };

            let difficulty = skill.map(|skill| skill.dodge_difficulty).unwrap_or(0.0);
            let dodge_chance =
                target_stats.dodge_chance(target_manual_dodge.is_some(), &difficulty);

            let target_number = rand::random::<f32>();

            if target_number <= dodge_chance {
                if let CombatEventTrigger::Skill(_) = &event.trigger {
                    events_to_send.push(CombatEvent {
                        source: event.source,
                        trigger: event.trigger.clone(),
                        kind: CombatEventKind::ExecuteScripts(ExecutionPhase::OnDodge),
                    });
                }

                if target_manual_dodge.is_some() {
                    bevy.entity(source_combat_state.target)
                        .remove::<ManualDodge>();
                }
            } else {
                events_to_send.push(CombatEvent {
                    source: event.source,
                    trigger: event.trigger.clone(),
                    kind: CombatEventKind::AttemptBlock,
                });
            }
        }
    }

    for event in events_to_send {
        combat_events.p1().send(event);
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_attempt_block(
    mut bevy: Commands,
    mut combat_events: ParamSet<(EventReader<CombatEvent>, EventWriter<CombatEvent>)>,
    fighters: Query<(
        &CombatState,
        &Stats,
        Option<&ManualBlock>,
        Option<&BlockCooldown>,
    )>,
) -> Result<(), anyhow::Error> {
    let mut events_to_send: Vec<CombatEvent> = vec![];

    for event in combat_events.p0().iter() {
        if let CombatEventKind::AttemptBlock = &event.kind {
            let (source_combat_state, _, _, _) = fighters.get(event.source)?;
            let (_, target_stats, target_manual_block, target_dodge_timer) =
                fighters.get(source_combat_state.target)?;

            if target_dodge_timer.is_some() {
                continue;
            }

            let skill = match &event.trigger {
                CombatEventTrigger::Skill(skill) => Some(skill),
                _ => None,
            };

            let difficulty = skill.map(|skill| skill.block_difficulty).unwrap_or(0.0);
            let block_chance =
                target_stats.block_chance(target_manual_block.is_some(), &difficulty);

            let target_number = rand::random::<f32>();

            if target_number <= block_chance {
                if let CombatEventTrigger::Skill(_) = &event.trigger {
                    events_to_send.push(CombatEvent {
                        source: event.source,
                        trigger: event.trigger.clone(),
                        kind: CombatEventKind::ExecuteScripts(ExecutionPhase::OnBlock),
                    });
                }

                if target_manual_block.is_some() {
                    bevy.entity(source_combat_state.target)
                        .remove::<ManualBlock>();
                }
            } else {
                events_to_send.push(CombatEvent {
                    source: event.source,
                    trigger: event.trigger.clone(),
                    kind: CombatEventKind::ExecuteScripts(ExecutionPhase::OnHit),
                });
            }
        }
    }

    for event in events_to_send {
        combat_events.p1().send(event);
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_execute_scripts(
    lua: NonSend<Lua>,
    mut event_reader: EventReader<CombatEvent>,
    mut executions: EventWriter<ExecutionEvent>,
    combat_states: Query<&CombatState>,
) -> Result<(), anyhow::Error> {
    for event in event_reader.iter() {
        if let CombatEventKind::ExecuteScripts(phase) = &event.kind {
            match &event.trigger {
                CombatEventTrigger::Skill(skill) => {
                    let combat_state = combat_states.get(event.source)?;

                    executions.send(ExecutionEvent {
                        context: ExecutionContext::with_sandbox(
                            &lua,
                            ExecutionKind::Skill(skill.clone()),
                            event.source,
                            combat_state.target,
                        ),
                        scripts: skill.scripts.clone(),
                        phase: phase.clone(),
                    });
                }
                CombatEventTrigger::Condition(condition) => {
                    executions.send(ExecutionEvent {
                        context: ExecutionContext::with_sandbox(
                            &lua,
                            ExecutionKind::Condition(condition.clone()),
                            event.source,
                            event.source,
                        ),
                        scripts: condition.scripts.clone(),
                        phase: phase.clone(),
                    });
                }
                _ => (),
            }
        }
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_attempt_flee(
    mut bevy: Commands,
    mut event_reader: EventReader<CombatEvent>,
    mut movement_events: EventWriter<MovementEvent>,
    fighters: Query<(Option<&Client>, &CombatState, &Stats, Option<&FleeTimer>)>,
    mut outbox: EventWriter<Outbox>,
) -> Result<(), anyhow::Error> {
    for event in event_reader.iter() {
        if let CombatEventKind::AttemptFlee(direction) = &event.kind {
            let (source_client, source_combat_state, source_stats, source_flee_timer) =
                fighters.get(event.source)?;
            let (_, _, target_stats, _) = fighters.get(source_combat_state.target)?;

            let client =
                source_client.with_context(|| "Failed to find client for fleeing player")?;

            if source_flee_timer.is_some() {
                outbox.send_text(client.id, "You are not ready to flee again.");

                continue;
            }

            let flee_chance = source_stats.flee_chance(&(target_stats.offense.dominance as f32));

            let target_number = rand::random::<f32>();

            bevy.entity(event.source)
                .insert(FleeTimer(Timer::from_seconds(
                    FLEE_COOLDOWN,
                    TimerMode::Once,
                )));

            if flee_chance >= target_number {
                movement_events.send(MovementEvent {
                    source: event.source,
                    kind: MovementEventKind::Flee(direction.clone()),
                })
            } else {
                outbox.send_text(client.id, "You failed to get away.");
            }
        }
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_apply_damage(
    mut events: EventReader<CombatEvent>,
    mut fighters: Query<(&CombatState, &mut Stats, &Modifiers, Option<&Client>)>,
    damage_kinds: Res<DamageKinds>,
    mut response: EventWriter<ApplyDamageResponse>,
    mut prompts: EventWriter<Prompt>,
) -> Result<(), anyhow::Error> {
    let mut targets_to_damage: Vec<(Entity, u32, String, bool, &Option<WithCallback>)> = vec![];
    for event in events.iter() {
        if let CombatEventKind::ApplyDamage(args) = &event.kind {
            let (source_combat_state, source_stats, source_modifiers, _) =
                fighters.get(event.source)?;
            let (_, target_stats, _, _) = fighters.get(args.target)?;

            let mut damage = f32::floor(args.damage) as u32;
            let mut crit = false;

            let critical_strike_chance = source_stats.critical_strike_chance()
                + source_modifiers.sum_stat(&Stat::CritStrikeChance);

            let target_number = rand::random::<f32>();

            if target_number <= critical_strike_chance {
                let critical_strike_damage = source_stats.critical_strike_damage();

                damage += f32::floor(critical_strike_damage) as u32;

                crit = true;
            }

            let kind = damage_kinds
                .0
                .get(&args.kind)
                .with_context(|| format!("Failed to find damage type: {}", &args.kind))?;

            let resisted = target_stats.resisted(kind);

            damage *= 1 - resisted;

            targets_to_damage.push((
                source_combat_state.target,
                damage,
                args.kind.clone(),
                crit,
                &args.with_callback,
            ));
        }
    }

    for (target, damage, kind, crit, callback) in targets_to_damage {
        let (_, mut stats, _, client) = fighters.get_mut(target)?;

        stats.status.health = stats.status.health.saturating_sub(damage);

        if let Some(with_callback) = callback {
            response.send(ApplyDamageResponse {
                context: with_callback.context.clone(),
                callback_id: with_callback.callback_id,
                damage,
                kind,
                crit,
            });
        }

        if let Some(client) = client {
            prompts.send(Prompt::new(client.id));
        }
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_apply_condition(
    mut combat_events: ParamSet<(EventReader<CombatEvent>, EventWriter<CombatEvent>)>,
    mut fighters: Query<(Entity, &mut Conditions)>,
    conditions: Res<data::resources::Conditions>,
) -> Result<(), anyhow::Error> {
    let mut events_to_send: Vec<CombatEvent> = vec![];
    let mut events = combat_events.p0();

    for event in events.iter() {
        if let CombatEventKind::ApplyCondition(args) = &event.kind {
            let condition = conditions
                .0
                .get(&args.condition)
                .with_context(|| format!("Failed to find condition: {}", args.condition))?;

            let (_, _) = fighters.get(event.source)?;
            let (_, mut target_conditions) = fighters.get_mut(args.target)?;

            target_conditions.0.insert(
                args.condition.clone(),
                args.duration
                    .map(|duration| Timer::from_seconds(duration, TimerMode::Once)),
            );

            events_to_send.push(CombatEvent {
                source: event.source,
                trigger: CombatEventTrigger::Condition(condition.clone()),
                kind: CombatEventKind::ExecuteCondition(ExecutionPhase::OnInit),
            });
        }
    }

    for event in events_to_send {
        combat_events.p1().send(event);
    }

    Ok(())
}

#[sysfail(log)]
pub fn update_condition_timer(
    conditions: Res<data::resources::Conditions>,
    mut events: EventWriter<CombatEvent>,
    mut timers: Query<(Entity, &mut Conditions)>,
    time: Res<Time>,
) -> Result<(), anyhow::Error> {
    for (entity, mut applied_conditions) in timers.iter_mut() {
        let mut conditions_to_remove: Vec<String> = vec![];

        for (condition, timer) in applied_conditions.0.iter_mut() {
            if let Some(timer) = timer {
                timer.tick(time.delta());

                if timer.finished() {
                    conditions_to_remove.push(condition.clone());
                }
            }
        }

        for condition in conditions_to_remove {
            applied_conditions.0.remove(&condition);

            let condition = conditions
                .0
                .get(&condition)
                .with_context(|| format!("Failed to find condition: {}", condition))?;

            events.send(CombatEvent {
                source: entity,
                trigger: CombatEventTrigger::Condition(condition.clone()),
                kind: CombatEventKind::ExecuteCondition(ExecutionPhase::OnEnd),
            });
        }
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_set_distance(
    mut event_reader: EventReader<CombatEvent>,
    mut fighters: Query<&mut CombatState>,
) -> Result<(), anyhow::Error> {
    let mut entities_to_update: Vec<(Entity, Distance)> = vec![];

    for event in event_reader.iter() {
        if let CombatEventKind::SetDistance(args) = &event.kind {
            let combat_state = fighters.get_mut(args.target)?;

            entities_to_update.push((args.target, args.distance));
            entities_to_update.push((combat_state.target, args.distance));
        }
    }

    for (entity, distance) in entities_to_update {
        let mut combat_state = fighters.get_mut(entity)?;

        combat_state.distance = distance;
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_set_approach(
    mut event_reader: EventReader<CombatEvent>,
    mut fighters: Query<&mut CombatState>,
) -> Result<(), anyhow::Error> {
    for event in event_reader.iter() {
        if let CombatEventKind::SetApproach(args) = &event.kind {
            let mut combat_state = fighters.get_mut(args.target)?;

            combat_state.approach = args.approach;
        }
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_add_stat_modifier(
    mut event_reader: EventReader<CombatEvent>,
    mut fighters: Query<&mut Modifiers>,
) -> Result<(), anyhow::Error> {
    for event in event_reader.iter() {
        if let CombatEventKind::AddStatModifier(args) = &event.kind {
            let mut modifiers = fighters.get_mut(args.target)?;

            modifiers
                .0
                .insert(args.id.clone(), (args.stat.clone(), args.amount));
        }
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_remove_stat_modifier(
    mut event_reader: EventReader<CombatEvent>,
    mut fighters: Query<&mut Modifiers>,
) -> Result<(), anyhow::Error> {
    for event in event_reader.iter() {
        if let CombatEventKind::RemoveStatModifier(args) = &event.kind {
            let mut modifiers = fighters.get_mut(args.target)?;

            modifiers.0.remove(&args.id);
        }
    }

    Ok(())
}

#[sysfail(log)]
pub fn on_combat_event_combat_log(
    mut events: EventReader<CombatEvent>,
    fighters: Query<(Option<&Client>, Option<&Character>, Option<&Depiction>)>,
    mut outbox: EventWriter<Outbox>,
) -> Result<(), anyhow::Error> {
    for event in events.iter() {
        if let CombatEventKind::CombatLog(log) = &event.kind {
            let (_, source_character, source_depiction) = fighters.get(log.source)?;
            let (target_client, _, _) = fighters.get(log.target)?;

            if let Some(client) = target_client {
                let name = source_character
                    .map(|character| format!("<fg.player>{}</>", character.name))
                    .unwrap_or_else(|| {
                        source_depiction
                            .map(|depiction| format!("<fg.hostile>{}</>", depiction.name))
                            .unwrap_or_else(|| "Unknown".to_string())
                    });

                match &log.kind {
                    CombatLogKind::Used(used) => {
                        let message = used.message.clone();

                        outbox.send_text(client.id, paint!("<{name}> {message}"));
                    }
                    CombatLogKind::Missed(missed) => {
                        let message = missed.message.clone();

                        outbox.send_text(client.id, paint!("<{name} /> {message}"));
                    }
                    CombatLogKind::Dodged(dodged) => {
                        let message = dodged.message.clone();

                        outbox.send_text(client.id, paint!("<{name} ~> {message}"));
                    }
                    CombatLogKind::Blocked(blocked) => {
                        let message = blocked.message.clone();

                        if let Some(damage) = blocked.damage {
                            outbox.send_text(
                                client.id,
                                paint!("<{name} <fg.red>{damage}d</> ~> {message}"),
                            );
                        } else {
                            outbox.send_text(client.id, paint!("<{name} ~> {message}"));
                        }
                    }
                    CombatLogKind::Damaged(damaged) => {
                        let damage = damaged.damage;
                        let kind = damaged.kind.to_title_case();
                        let crit = if damaged.crit { "!" } else { "" };
                        let message = damaged.message.clone();

                        outbox.send_text(
                            client.id,
                            paint!(
                                "<{name} <fg.red>{damage}d</>{crit}, <fg.yellow>{kind}</>> {message}"
                            ),
                        );
                    }
                    CombatLogKind::ConditionApplied(condition_applied) => {
                        let message = condition_applied.message.clone();
                        let condition = condition_applied.condition.clone();

                        outbox.send_text(
                            client.id,
                            paint!("<{name} <fg.yellow>+{condition}+</>> {message}"),
                        );
                    }
                    CombatLogKind::ConditionRemoved(condition_removed) => {
                        let message = condition_removed.message.clone();
                        let condition = condition_removed.condition.clone();

                        outbox.send_text(
                            client.id,
                            paint!("<{name} <fg.yellow>-{condition}-</>> {message}"),
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn update_attack_timer(
    mut bevy: Commands,
    mut proxy: EventWriter<ProxyCommand>,
    mut timers: Query<(
        Entity,
        &mut AttackTimer,
        Option<&QueuedAttack>,
        Option<&Client>,
    )>,
    time: Res<Time>,
    mut outbox: EventWriter<Outbox>,
) {
    for (entity, mut attack_timer, queued_attack, client) in timers.iter_mut() {
        attack_timer.0.tick(time.delta());

        if attack_timer.0.finished() {
            bevy.entity(entity).remove::<AttackTimer>();

            match queued_attack {
                Some(queued_attack) => {
                    proxy.send(ProxyCommand(queued_attack.0.clone()));
                    bevy.entity(entity).remove::<QueuedAttack>();
                }
                None => {
                    if let Some(client) = client {
                        outbox.send_text(client.id, "You are ready to attack again.");
                    }
                }
            }
        }
    }
}

pub fn update_dodge_timer(
    mut bevy: Commands,
    mut timers: Query<(Entity, &mut DodgeCooldown)>,
    time: Res<Time>,
) {
    for (entity, mut dodge_timer) in timers.iter_mut() {
        dodge_timer.0.tick(time.delta());

        if dodge_timer.0.finished() {
            bevy.entity(entity).remove::<DodgeCooldown>();
        }
    }
}

pub fn update_manual_dodge_timer(
    mut bevy: Commands,
    mut timers: Query<(Entity, &mut ManualDodge)>,
    time: Res<Time>,
) {
    for (entity, mut dodge_timer) in timers.iter_mut() {
        dodge_timer.0.tick(time.delta());

        if dodge_timer.0.finished() {
            bevy.entity(entity).remove::<ManualDodge>();
        }
    }
}

pub fn update_block_timer(
    mut bevy: Commands,
    mut timers: Query<(Entity, &mut BlockCooldown)>,
    time: Res<Time>,
) {
    for (entity, mut block_timer) in timers.iter_mut() {
        block_timer.0.tick(time.delta());

        if block_timer.0.finished() {
            bevy.entity(entity).remove::<BlockCooldown>();
        }
    }
}

pub fn update_manual_block_timer(
    mut bevy: Commands,
    mut timers: Query<(Entity, &mut ManualBlock)>,
    time: Res<Time>,
) {
    for (entity, mut block_timer) in timers.iter_mut() {
        block_timer.0.tick(time.delta());

        if block_timer.0.finished() {
            bevy.entity(entity).remove::<ManualBlock>();
        }
    }
}

pub fn update_flee_timer(
    mut bevy: Commands,
    mut timers: Query<(Entity, &mut FleeTimer)>,
    time: Res<Time>,
) {
    for (entity, mut flee_timer) in timers.iter_mut() {
        flee_timer.0.tick(time.delta());

        if flee_timer.0.finished() {
            bevy.entity(entity).remove::<FleeTimer>();
        }
    }
}

pub fn health_regen(time: Res<Time>, mut timers: Query<(&mut Stats, &mut HealthRegenTimer)>) {
    for (mut stats, mut timer) in timers.iter_mut() {
        if timer.0.tick(time.delta()).just_finished() {
            stats.status.health = u32::min(
                stats.status.health + stats.health_per_second(),
                stats.max_health(),
            );
        }
    }
}

pub fn vigor_regen(time: Res<Time>, mut timers: Query<(&mut Stats, &mut VigorRegenTimer)>) {
    for (mut stats, mut timer) in timers.iter_mut() {
        if timer.0.tick(time.delta()).just_finished() {
            stats.status.vigor = u32::min(
                stats.status.vigor + stats.vigor_per_second(),
                stats.max_vigor(),
            );
        }
    }
}

#[sysfail(log)]
pub fn update_cooldowns(
    mut outbox: EventWriter<Outbox>,
    time: Res<Time>,
    mut cooldowns: Query<&mut Cooldowns>,
    clients: Query<&Client>,
    skills: Res<Skills>,
) -> Result<(), anyhow::Error> {
    for mut cooldowns in cooldowns.iter_mut() {
        let finished: Vec<(String, Entity)> = cooldowns
            .0
            .iter_mut()
            .filter_map(|(skill, (entity, timer))| {
                if timer.tick(time.delta()).just_finished() {
                    Some((skill.clone(), *entity))
                } else {
                    None
                }
            })
            .collect();

        for (skill, entity) in finished {
            cooldowns.0.remove(&skill);

            if let Ok(client) = clients.get(entity) {
                let skill = skills
                    .0
                    .get(&skill)
                    .with_context(|| format!("Failed to find skill with id: {}", &skill))?;

                outbox.send_text(client.id, format!("{} is ready.", skill.name));
            }
        }
    }

    Ok(())
}

pub fn on_hostile_death(
    mut bevy: Commands,
    mut outbox: EventWriter<Outbox>,
    hostiles: Query<(Entity, &Depiction, &Stats, &Parent), With<Hostile>>,
    mut players: Query<(Entity, &Client, &CombatState), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) {
    for (entity, depiction, stats, parent) in hostiles.iter() {
        let siblings = tiles.get(parent.get()).ok();

        if stats.status.health == 0 {
            let players_in_combat = players
                .iter_mut()
                .filter(|(_, _, combat_state)| combat_state.target == entity);

            for (player, _, _) in players_in_combat {
                bevy.entity(player).remove::<CombatState>();
            }

            let players_on_tile = siblings
                .map(|siblings| {
                    siblings
                        .iter()
                        .filter_map(|entity| players.get(*entity).ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            for (_, client, _) in players_on_tile {
                outbox.send_text(client.id, format!("{} has died.", depiction.name));
            }

            bevy.entity(entity).despawn();
        }
    }
}

pub fn on_player_death(
    mut bevy: Commands,
    mut hostiles: Query<(Entity, &CombatState), With<Hostile>>,
    mut outbox: EventWriter<Outbox>,
    mut proxy: EventWriter<ProxyCommand>,
    mut players: Query<(Entity, &Client, &mut Stats), (With<Online>, With<CombatState>)>,
    spawn_tiles: Query<Entity, With<DeathSpawn>>,
) {
    for (player, client, mut stats) in players.iter_mut() {
        if stats.status.health == 0 {
            outbox.send_text(client.id, "You have died.");

            bevy.entity(player).remove::<CombatState>();
            bevy.entity(player).remove::<QueuedAttack>();

            stats.status.health = stats.max_health();

            let hostiles_in_combat = hostiles
                .iter_mut()
                .filter(|(_, combat_state)| combat_state.target == player);

            for (entity, _) in hostiles_in_combat {
                bevy.entity(entity).remove::<CombatState>();
            }

            if let Some(tile) = spawn_tiles.iter().next() {
                bevy.entity(player).set_parent(tile);

                proxy.send(ProxyCommand(ParsedCommand {
                    from: client.id,
                    command: Command::Look(None),
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use std::time::Duration;

    use crate::test::tile_builder::{TileBuilder, ZoneBuilder};
    use crate::test::utils::get_message_content;
    use crate::test::{
        app_builder::AppBuilder, npc_builder::NpcBuilder, player_builder::PlayerBuilder,
    };

    use crate::combat::components::{Approach, Distance};

    use super::*;

    #[fixture]
    fn setup() -> (App, Entity, ClientId, Entity) {
        let mut app = AppBuilder::new().build();

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        let npc = NpcBuilder::new()
            .name("Goat")
            .combat(true)
            .tile(tile)
            .build(&mut app);

        app.world.entity_mut(player).insert(CombatState {
            target: npc,
            distance: Distance::Near,
            approach: Approach::Front,
        });

        (app, player, client_id, npc)
    }

    #[rstest]
    fn update_attack_timer_removes_component(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, player, _, _) = setup;
        app.add_systems(Update, update_attack_timer);

        let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
        timer.set_elapsed(Duration::from_secs(1));
        app.world.entity_mut(player).insert(AttackTimer(timer));

        app.update();

        assert!(app.world.get::<AttackTimer>(player).is_none());
    }

    #[rstest]
    fn on_npc_death_destroys_entity(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, _, _, npc) = setup;
        app.add_systems(Update, on_hostile_death);

        app.world.entity_mut(npc).insert(Stats::default());
        app.update();

        assert!(app.world.get_entity(npc).is_none());
    }

    #[rstest]
    fn on_npc_death_alerts_neighbors(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, _, client_id, npc) = setup;
        app.add_systems(Update, on_hostile_death);

        app.world.entity_mut(npc).insert(Stats::default());

        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Goat has died.");
    }

    #[rstest]
    fn on_player_death_resets_state(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, player, _, _) = setup;
        app.add_systems(Update, on_player_death);

        app.world.entity_mut(player).insert(Stats::default());
        app.update();

        assert!(app.world.get::<CombatState>(player).is_none());

        assert_eq!(
            app.world.get::<Stats>(player).unwrap().status.health,
            app.world.get::<Stats>(player).unwrap().max_health()
        );
    }

    #[rstest]
    fn on_player_death_teleports_player(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, player, _, _) = setup;
        app.add_systems(Update, on_player_death);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        app.world.entity_mut(tile).insert(DeathSpawn);

        app.world.entity_mut(player).insert(Stats::default());
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), tile);
    }
}
