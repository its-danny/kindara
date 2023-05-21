use bevy::prelude::*;
use sqlx::PgPool;

#[derive(Resource)]
pub struct DatabasePool(pub PgPool);
