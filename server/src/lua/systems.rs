use anyhow::Context;
use bevy::{asset::FileAssetIo, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use mlua::{prelude::*, Function, Table, Value};
use strum::IntoEnumIterator;
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{
    combat::{
        components::{Approach, CombatState, Distance, Stats},
        events::{
            AddStatModifier, ApplyCondition, ApplyDamage, Blocked, CombatEvent, CombatEventKind,
            CombatEventTrigger, CombatLog, CombatLogKind, ConditionApplied, ConditionRemoved,
            Damaged, Dodged, Missed, RemoveStatModifier, SetApproach, SetDistance, Used,
            WithCallback,
        },
    },
    data::resources::Stat,
    player::components::Client,
};

use super::{
    context::{ExecutionContext, ExecutionKind},
    data::LuaEntity,
    events::{Action, ActionEvent, ApplyDamageResponse, ExecutionEvent, ProcessEvent, SendMessage},
    resources::Scripts,
};

pub fn load_scripts(mut scripts: ResMut<Scripts>) {
    let path = FileAssetIo::get_base_path().join("assets");

    debug!("Loading Lua scripts from: {:?}", path);

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| e.path().extension().unwrap() == "lua")
    {
        let path = entry.path();

        let contents = std::fs::read_to_string(path)
            .expect("Failed to load script")
            .as_str()
            .to_string();

        let name = path.file_stem().unwrap().to_str().unwrap().to_string();

        scripts.0.insert(name, contents);
    }
}

#[sysfail(log)]
pub fn execute_scripts(
    mut executions: EventReader<ExecutionEvent>,
    mut process: EventWriter<ProcessEvent>,
    scripts: Res<Scripts>,
    stats: Query<&Stats>,
    combat_states: Query<&CombatState>,
    lua: NonSend<Lua>,
) -> Result<(), anyhow::Error> {
    for event in executions.iter() {
        for key in &event.scripts {
            let sandbox_id = event.context.sandbox_id.to_string();

            let globals = lua.globals();
            let sandboxes = globals.get::<_, Table>("sandboxes")?;
            let sandbox = sandboxes.get::<_, Table>(sandbox_id)?;

            for pair in globals.pairs::<String, Value>() {
                let (key, value) = pair?;

                if sandbox.get::<_, Option<Value>>(key.clone())?.is_none() {
                    sandbox.set(key, value)?;
                }
            }

            let events = lua.create_table()?;
            sandbox.set("events", events.clone())?;

            let callbacks = lua.create_table()?;
            sandbox.set("callbacks", callbacks.clone())?;

            let action = lua.create_table()?;

            action.set(
                "apply_damage",
                apply_damage_func(
                    &lua,
                    event.context.sandbox_id.to_string(),
                    event.context.clone(),
                )?,
            )?;

            action.set(
                "apply_condition",
                apply_condition_func(&lua, event.context.sandbox_id.to_string())?,
            )?;

            action.set(
                "set_distance",
                set_distance_func(&lua, event.context.sandbox_id.to_string())?,
            )?;

            action.set(
                "set_approach",
                set_approach_func(&lua, event.context.sandbox_id.to_string())?,
            )?;

            action.set(
                "add_stat_modifier",
                add_stat_modifier_func(&lua, event.context.sandbox_id.to_string())?,
            )?;

            action.set(
                "remove_stat_modifier",
                remove_stat_modifier_func(&lua, event.context.sandbox_id.to_string())?,
            )?;

            action.set(
                "combat_log",
                combat_log_func(&lua, event.context.sandbox_id.to_string())?,
            )?;

            action.set(
                "send_message",
                send_message_func(&lua, event.context.sandbox_id.to_string())?,
            )?;

            let var = lua.create_table()?;

            var.set(
                "source",
                combat_entity_var(&lua, event.context.source, &stats, &combat_states)?,
            )?;

            var.set(
                "target",
                combat_entity_var(&lua, event.context.target, &stats, &combat_states)?,
            )?;

            var.set("stat", stat_var(&lua)?)?;
            var.set("distance", distance_var(&lua)?)?;
            var.set("approach", approach_var(&lua)?)?;

            let script = scripts
                .0
                .get(key)
                .with_context(|| format!("Failed to find script: {}", key))?;

            let result: Table = lua.load(script.as_str()).set_environment(sandbox).eval()?;

            if let Some(phase) = result.get::<_, Option<LuaFunction>>(event.phase.to_string())? {
                phase.call::<_, ()>((result, action, var))?;
            }

            process.send(ProcessEvent {
                context: event.context.clone(),
            });
        }
    }

    Ok(())
}

/**
Usage:
```lua
action.apply_damage(var.target.entity, {
    damage = 10,
    kind = "physical",
    after = function(damage, kind, crit)
        print("Damage applied: " .. damage .. " (" .. kind .. ")")
    end
})
```
*/
fn apply_damage_func(
    lua: &Lua,
    sandbox_id: String,
    context: ExecutionContext,
) -> mlua::Result<Function> {
    let func = lua.create_function(move |ctx, args: (LuaEntity, Table)| {
        let sandboxes: Table = ctx.globals().get::<_, Table>("sandboxes")?;
        let sandbox: Table = sandboxes.get::<_, Table>(sandbox_id.clone())?;

        let (target, args) = args;

        let damage: f32 = args.get("damage")?;
        let kind: String = args.get("kind")?;
        let after: Option<Function> = args.get("after")?;

        let after = match after {
            Some(callback) => {
                let uuid = Uuid::new_v4();
                let callbacks: Table = sandbox.get("callbacks")?;

                callbacks.set(uuid.to_string(), callback)?;

                Some(uuid)
            }
            None => None,
        };

        let events: Table = sandbox.get("events")?;

        events.set(
            events.len()? + 1,
            Action::ApplyDamage(ApplyDamage {
                target: target.0,
                kind,
                damage,
                with_callback: after.map(|uuid| WithCallback {
                    context: context.clone(),
                    callback_id: uuid,
                }),
            }),
        )?;

        Ok(())
    })?;

    Ok(func)
}

/**
Usage:
```lua
action.apply_condition(var.target.entity, {
    id = "stunned",
    duration = 5
})
```
*/
fn apply_condition_func(lua: &Lua, sandbox_id: String) -> mlua::Result<Function> {
    let func = lua.create_function(move |ctx, args: (LuaEntity, Table)| {
        let sandboxes: Table = ctx.globals().get::<_, Table>("sandboxes")?;
        let sandbox: Table = sandboxes.get::<_, Table>(sandbox_id.clone())?;

        let (target, args) = args;

        let condition = args.get::<_, String>("id")?;
        let duration = args.get::<_, f32>("duration");

        let events: Table = sandbox.get("events")?;

        events.set(
            events.len()? + 1,
            Action::ApplyCondition(ApplyCondition {
                target: target.0,
                condition,
                duration: duration.ok(),
            }),
        )?;

        Ok(())
    })?;

    Ok(func)
}

/**
Usage:
```lua
action.set_distance(var.target.entity, {
    distance = distance.Near
})
```
*/
fn set_distance_func(lua: &Lua, sandbox_id: String) -> mlua::Result<Function> {
    let func = lua.create_function(move |ctx, args: (LuaEntity, Table)| {
        let sandboxes: Table = ctx.globals().get::<_, Table>("sandboxes")?;
        let sandbox: Table = sandboxes.get::<_, Table>(sandbox_id.clone())?;

        let (target, args) = args;

        let distance = args.get::<_, Distance>("distance")?;

        let events: Table = sandbox.get("events")?;

        events.set(
            events.len()? + 1,
            Action::SetDistance(SetDistance {
                target: target.0,
                distance,
            }),
        )?;

        Ok(())
    })?;

    Ok(func)
}

/**
Usage:
```lua
action.set_approach(var.target.entity, {
    approach = approach.Front
})
```
*/
fn set_approach_func(lua: &Lua, sandbox_id: String) -> mlua::Result<Function> {
    let func = lua.create_function(move |ctx, args: (LuaEntity, Table)| {
        let sandboxes: Table = ctx.globals().get::<_, Table>("sandboxes")?;
        let sandbox: Table = sandboxes.get::<_, Table>(sandbox_id.clone())?;

        let (target, args) = args;

        let approach = args.get::<_, Approach>("approach")?;

        let events: Table = sandbox.get("events")?;

        events.set(
            events.len()? + 1,
            Action::SetApproach(SetApproach {
                target: target.0,
                approach,
            }),
        )?;

        Ok(())
    })?;

    Ok(func)
}

/**
Usage:
```lua
action.add_stat_modifier(var.target.entity, {
    stat = stat.Strength,
    amount = 5
})
```
*/
fn add_stat_modifier_func(lua: &Lua, sandbox_id: String) -> mlua::Result<Function> {
    let func = lua.create_function(move |ctx, args: (LuaEntity, Table)| {
        let sandboxes: Table = ctx.globals().get::<_, Table>("sandboxes")?;
        let sandbox: Table = sandboxes.get::<_, Table>(sandbox_id.clone())?;

        let (target, args) = args;

        let stat = args.get::<_, Stat>("stat")?;
        let amount = args.get::<_, f32>("amount")?;

        let events: Table = sandbox.get("events")?;
        let uuid = Uuid::new_v4();

        events.set(
            events.len()? + 1,
            Action::AddStatModifier(AddStatModifier {
                target: target.0,
                id: uuid.to_string(),
                stat,
                amount,
            }),
        )?;

        Ok(uuid.to_string())
    })?;

    Ok(func)
}

/**
Usage:
```lua
action.remove_stat_modifier(var.target.entity, {
    id = "some-uuid"
})
```
*/
fn remove_stat_modifier_func(lua: &Lua, sandbox_id: String) -> mlua::Result<Function> {
    let func = lua.create_function(move |ctx, args: (LuaEntity, Table)| {
        let sandboxes: Table = ctx.globals().get::<_, Table>("sandboxes")?;
        let sandbox: Table = sandboxes.get::<_, Table>(sandbox_id.clone())?;

        let (target, args) = args;

        let id = args.get::<_, String>("id")?;

        let events: Table = sandbox.get("events")?;

        events.set(
            events.len()? + 1,
            Action::RemoveStatModifier(RemoveStatModifier {
                target: target.0,
                id,
            }),
        )?;

        Ok(())
    })?;

    Ok(func)
}

/**
Usage:
```lua
action.combat_log(var.source.entity, var.target.entity, {
    damaged = { message = "They hit ya!", damage = 10, kind = "physical", crit = true },
})
```
*/
fn combat_log_func(lua: &Lua, sandbox_id: String) -> mlua::Result<Function> {
    let func = lua.create_function(move |ctx, args: (LuaEntity, LuaEntity, Table)| {
        let sandboxes = ctx.globals().get::<_, Table>("sandboxes")?;
        let sandbox = sandboxes.get::<_, Table>(sandbox_id.clone())?;

        let (source, target, args) = args;

        if let Ok(used) = args.get::<_, Table>("used") {
            let message = used.get::<_, String>("message")?;

            let events = sandbox.get::<_, Table>("events")?;

            events.set(
                events.len()? + 1,
                Action::CombatLog(CombatLog {
                    source: source.0,
                    target: target.0,
                    kind: CombatLogKind::Used(Used {
                        message: message.clone(),
                    }),
                }),
            )?;
        }

        if let Ok(missed) = args.get::<_, Table>("missed") {
            let message = missed.get::<_, String>("message")?;

            let events = sandbox.get::<_, Table>("events")?;

            events.set(
                events.len()? + 1,
                Action::CombatLog(CombatLog {
                    source: source.0,
                    target: target.0,
                    kind: CombatLogKind::Missed(Missed {
                        message: message.clone(),
                    }),
                }),
            )?;
        }

        if let Ok(dodged) = args.get::<_, Table>("dodged") {
            let message = dodged.get::<_, String>("message")?;

            let events = sandbox.get::<_, Table>("events")?;

            events.set(
                events.len()? + 1,
                Action::CombatLog(CombatLog {
                    source: source.0,
                    target: target.0,
                    kind: CombatLogKind::Dodged(Dodged {
                        message: message.clone(),
                    }),
                }),
            )?;
        }

        if let Ok(blocked) = args.get::<_, Table>("blocked") {
            let message = blocked.get::<_, String>("message")?;
            let damage = blocked.get::<_, u32>("damage");

            let events = sandbox.get::<_, Table>("events")?;

            events.set(
                events.len()? + 1,
                Action::CombatLog(CombatLog {
                    source: source.0,
                    target: target.0,
                    kind: CombatLogKind::Blocked(Blocked {
                        message: message.clone(),
                        damage: damage.ok(),
                    }),
                }),
            )?;
        }

        if let Ok(damaged) = args.get::<_, Table>("damaged") {
            let message = damaged.get::<_, String>("message")?;
            let damage = damaged.get::<_, i32>("damage")?;
            let kind = damaged.get::<_, String>("kind")?;
            let crit = damaged.get::<_, bool>("crit")?;

            let events = sandbox.get::<_, Table>("events")?;

            events.set(
                events.len()? + 1,
                Action::CombatLog(CombatLog {
                    source: source.0,
                    target: target.0,
                    kind: CombatLogKind::Damaged(Damaged {
                        message: message.clone(),
                        damage: damage as u32,
                        kind,
                        crit,
                    }),
                }),
            )?;
        }

        if let Ok(condition_applied) = args.get::<_, Table>("condition_applied") {
            let message = condition_applied.get::<_, String>("message")?;
            let condition = condition_applied.get::<_, String>("condition")?;

            let events = sandbox.get::<_, Table>("events")?;

            events.set(
                events.len()? + 1,
                Action::CombatLog(CombatLog {
                    source: source.0,
                    target: target.0,
                    kind: CombatLogKind::ConditionApplied(ConditionApplied {
                        message: message.clone(),
                        condition: condition.clone(),
                    }),
                }),
            )?;
        }

        if let Ok(condition_removed) = args.get::<_, Table>("condition_removed") {
            let message = condition_removed.get::<_, String>("message")?;
            let condition = condition_removed.get::<_, String>("condition")?;

            let events = sandbox.get::<_, Table>("events")?;

            events.set(
                events.len()? + 1,
                Action::CombatLog(CombatLog {
                    source: source.0,
                    target: target.0,
                    kind: CombatLogKind::ConditionRemoved(ConditionRemoved {
                        message: message.clone(),
                        condition: condition.clone(),
                    }),
                }),
            )?;
        }

        Ok(())
    })?;

    Ok(func)
}

/**
Usage:
```lua
action.send_message(var.target.entity, {
    message = "Hello"
})
```
*/
fn send_message_func(lua: &Lua, sandbox_id: String) -> mlua::Result<Function> {
    let func = lua.create_function(move |ctx, args: (LuaEntity, Table)| {
        let sandboxes = ctx.globals().get::<_, Table>("sandboxes")?;
        let sandbox = sandboxes.get::<_, Table>(sandbox_id.clone())?;

        let (target, args) = args;

        let message = args.get::<_, String>("message")?;

        let events = sandbox.get::<_, Table>("events")?;

        events.set(
            events.len()? + 1,
            Action::SendMessage(SendMessage {
                target: target.0,
                message,
            }),
        )?;

        Ok(())
    })?;

    Ok(func)
}

/**
Returns a table with the following fields:
- entity: [`LuaEntity`]
- stats: Their [`Stats`]
- distance: Their current [`Distance`]
- approach: Their current [`Approach`]

Accessible via `var.source` and `var.target`.

Usage:
```lua
send_message(var.source.target, { message = "Hello" })
```
*/
fn combat_entity_var<'a>(
    lua: &'a Lua,
    entity: Entity,
    stats: &Query<&Stats>,
    combat_states: &Query<&CombatState>,
) -> Result<Table<'a>, anyhow::Error> {
    let table = lua.create_table()?;

    table.set("entity", LuaEntity(entity))?;
    table.set("stats", stats.get(entity)?.clone())?;
    table.set("distance", combat_states.get(entity)?.distance)?;
    table.set("approach", combat_states.get(entity)?.approach)?;

    Ok(table)
}

/**
See [`Stat`] for possible values.

Usage:
```lua
action.add_stat_modifier(var.target.entity, { stat = var.stat.Strength, amount = 5 })
```
*/
fn stat_var(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;

    for stat in Stat::iter() {
        table.set(stat.to_string(), stat)?;
    }

    Ok(table)
}

/**
See [`Distance`] for possible values.

Usage:
```lua
action.set_distance(var.target.entity, { distance = var.distance.Near })
```
*/
fn distance_var(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;

    table.set("Near", Distance::Near)?;
    table.set("Far", Distance::Far)?;

    Ok(table)
}

/**
See [`Approach`] for possible values.

Usage:
```lua
action.set_approach(var.target.entity, { approach = var.approach.Front })
```
*/
fn approach_var(lua: &Lua) -> mlua::Result<Table> {
    let table = lua.create_table()?;

    table.set("Front", Approach::Front)?;
    table.set("Rear", Approach::Rear)?;

    Ok(table)
}

#[sysfail(log)]
pub fn process_action_events(
    lua: NonSend<Lua>,
    mut process: EventReader<ProcessEvent>,
    mut actions: EventWriter<ActionEvent>,
) -> Result<(), anyhow::Error> {
    for event in process.iter() {
        let sandboxes = lua.globals().get::<_, Table>("sandboxes")?;
        let sandbox = sandboxes.get::<_, Table>(event.context.sandbox_id.to_string())?;
        let events = sandbox.get::<_, Table>("events")?;

        for pair in events.clone().pairs::<usize, Action>() {
            let (_, action) = pair?;

            actions.send(ActionEvent {
                context: event.context.clone(),
                action,
            });
        }

        events.clear()?;
    }

    Ok(())
}

#[sysfail(log)]
pub fn handle_combat_event_action(
    mut actions: EventReader<ActionEvent>,
    mut combat_events: EventWriter<CombatEvent>,
) -> Result<(), anyhow::Error> {
    for action in actions.iter() {
        combat_events.send(CombatEvent {
            source: action.context.source,
            kind: match &action.action {
                Action::ApplyDamage(args) => CombatEventKind::ApplyDamage(args.clone()),
                Action::SetDistance(args) => CombatEventKind::SetDistance(args.clone()),
                Action::SetApproach(args) => CombatEventKind::SetApproach(args.clone()),
                Action::ApplyCondition(args) => CombatEventKind::ApplyCondition(args.clone()),
                Action::AddStatModifier(args) => CombatEventKind::AddStatModifier(args.clone()),
                Action::RemoveStatModifier(args) => {
                    CombatEventKind::RemoveStatModifier(args.clone())
                }
                Action::CombatLog(args) => CombatEventKind::CombatLog(args.clone()),
                _ => continue,
            },
            trigger: match &action.context.kind {
                ExecutionKind::Skill(skill) => CombatEventTrigger::Skill(skill.clone()),
                ExecutionKind::Condition(condition) => {
                    CombatEventTrigger::Condition(condition.clone())
                }
            },
        });
    }

    Ok(())
}

pub fn handle_send_message_action(
    mut actions: EventReader<ActionEvent>,
    mut outbox: EventWriter<Outbox>,
    clients: Query<&Client>,
) {
    for action in actions.iter() {
        if let Action::SendMessage(args) = &action.action {
            if let Ok(client) = clients.get(args.target) {
                outbox.send_text(client.id, &args.message);
            }
        }
    }
}

#[sysfail(log)]
pub fn on_apply_damage_response(
    lua: NonSend<Lua>,
    mut responses: EventReader<ApplyDamageResponse>,
    mut process: EventWriter<ProcessEvent>,
) -> Result<(), anyhow::Error> {
    for response in responses.iter() {
        let globals = lua.globals();
        let sandboxes: Table = globals.get("sandboxes")?;
        let sandbox: Table = sandboxes.get(response.context.sandbox_id.to_string())?;
        let callbacks: Table = sandbox.get("callbacks")?;
        let callback: Function = callbacks.get(response.callback_id.to_string())?;

        callback.call::<_, ()>((response.damage, response.kind.clone(), response.crit))?;

        process.send(ProcessEvent {
            context: response.context.clone(),
        });
    }

    Ok(())
}
