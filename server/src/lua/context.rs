use bevy::prelude::*;
use mlua::{prelude::*, Table};
use uuid::Uuid;

use crate::data::resources::{Condition, Skill};

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub sandbox_id: Uuid,
    pub kind: ExecutionKind,
    pub source: Entity,
    pub target: Entity,
}

impl ExecutionContext {
    /// Creates a new execution context and
    /// registers it in the global sandboxes table.
    pub fn with_sandbox(lua: &Lua, kind: ExecutionKind, source: Entity, target: Entity) -> Self {
        let globals = lua.globals();
        let sandbox = lua.create_table().unwrap();
        let sandbox_id = Uuid::new_v4();

        if let Ok(sandboxes) = globals.get::<_, Table>("sandboxes") {
            sandboxes
                .set(sandbox_id.to_string(), sandbox.clone())
                .unwrap();
        } else {
            let sandboxes = lua.create_table().unwrap();

            sandboxes
                .set(sandbox_id.to_string(), sandbox.clone())
                .unwrap();

            globals.set("sandboxes", sandboxes).unwrap();
        }

        Self {
            sandbox_id,
            kind,
            source,
            target,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExecutionKind {
    Skill(Skill),
    Condition(Condition),
}
