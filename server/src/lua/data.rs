use bevy::prelude::*;
use mlua::{prelude::*, FromLua, UserData, UserDataFields, UserDataMethods, Value};

use crate::{
    combat::components::{
        Approach, Attributes, Defense, Distance, Offense, Resistance, Stats, Status,
    },
    data::resources::Stat,
};

use super::events::Action;

#[derive(Clone, Debug)]
pub struct LuaEntity(pub Entity);

impl UserData for LuaEntity {}

impl<'a> FromLua<'a> for LuaEntity {
    fn from_lua(value: Value<'a>, _: &'a Lua) -> LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|entity| entity.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "LuaEntity",
                message: Some("expected LuaEntity".into()),
            }),
        }
    }
}

impl UserData for Action {}

impl<'a> FromLua<'a> for Action {
    fn from_lua(value: Value<'a>, _: &'a Lua) -> LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|action| action.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "LuaAction",
                message: Some("expected LuaAction".into()),
            }),
        }
    }
}

impl UserData for Stats {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("level", |_, stats| Ok(stats.level));
        fields.add_field_method_get("attributes", |_, stats| Ok(stats.attributes.clone()));
        fields.add_field_method_get("status", |_, stats| Ok(stats.status.clone()));
        fields.add_field_method_get("defense", |_, stats| Ok(stats.defense.clone()));
        fields.add_field_method_get("offense", |_, stats| Ok(stats.offense.clone()));
        fields.add_field_method_get("resistance", |_, stats| Ok(stats.resistance.clone()));
    }

    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("auto_attack_damage", |_, stats, _: ()| {
            Ok(stats.auto_attack_damage())
        })
    }
}

impl UserData for Attributes {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("vitality", |_, attributes| Ok(attributes.vitality));
        fields.add_field_method_get("stamina", |_, attributes| Ok(attributes.stamina));
        fields.add_field_method_get("strength", |_, attributes| Ok(attributes.strength));
        fields.add_field_method_get("dexterity", |_, attributes| Ok(attributes.dexterity));
        fields.add_field_method_get("intelligence", |_, attributes| Ok(attributes.intelligence));
    }
}

impl UserData for Status {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("health", |_, status| Ok(status.health));
        fields.add_field_method_get("vigor", |_, status| Ok(status.vigor));
        fields.add_field_method_get("vigor_regen", |_, status| Ok(status.vigor_regen));
    }
}

impl UserData for Defense {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("dodge_chance", |_, defense| Ok(defense.dodge_chance));
        fields.add_field_method_get("dodge_rate", |_, defense| Ok(defense.dodge_rate));
        fields.add_field_method_get("block_chance", |_, defense| Ok(defense.block_chance));
        fields.add_field_method_get("block_rate", |_, defense| Ok(defense.block_rate));
        fields.add_field_method_get("fleet", |_, defense| Ok(defense.fleet));
    }
}

impl UserData for Offense {
    fn add_fields<'lua, F: UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field_method_get("attack_speed", |_, offense| Ok(offense.attack_speed));
        fields.add_field_method_get("dominance", |_, offense| Ok(offense.dominance));
        fields.add_field_method_get("crit_strike_chance", |_, offense| {
            Ok(offense.crit_strike_chance)
        });
        fields.add_field_method_get("crit_strike_damage", |_, offense| {
            Ok(offense.crit_strike_damage)
        });
    }
}

impl UserData for Resistance {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("get", |_, resistance, name: String| {
            match resistance.0.get(&name) {
                Some(resistance) => Ok(*resistance),
                None => Err(mlua::Error::RuntimeError(format!(
                    "Resistance {} not found",
                    name
                ))),
            }
        })
    }
}

impl UserData for Stat {}

impl<'a> FromLua<'a> for Stat {
    fn from_lua(value: LuaValue<'a>, _: &'a Lua) -> LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|stat| stat.clone()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Stat",
                message: Some("expected Stat".into()),
            }),
        }
    }
}

impl UserData for Distance {}

impl<'a> FromLua<'a> for Distance {
    fn from_lua(value: LuaValue<'a>, _: &'a Lua) -> LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|distance| *distance),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Distance",
                message: Some("expected Distance".into()),
            }),
        }
    }
}

impl UserData for Approach {}

impl<'a> FromLua<'a> for Approach {
    fn from_lua(value: LuaValue<'a>, _: &'a Lua) -> LuaResult<Self> {
        match value {
            Value::UserData(data) => data.borrow::<Self>().map(|distance| *distance),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "Approach",
                message: Some("expected Approach".into()),
            }),
        }
    }
}
