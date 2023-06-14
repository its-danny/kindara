use chrono::NaiveDateTime;
use sqlx::{types::Json, FromRow};

use crate::{player::config::CharacterConfig, world::resources::WorldState};

#[derive(FromRow)]
pub struct CharacterModel {
    pub config: Json<CharacterConfig>,
    pub created_at: NaiveDateTime,
    pub description: Option<String>,
    pub email: Option<String>,
    pub id: i64,
    pub name: String,
    pub password: String,
    pub role: i16,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, FromRow)]
pub struct WorldSaveModel {
    pub created_at: NaiveDateTime,
    pub id: i64,
    pub state: Json<WorldState>,
    pub updated_at: NaiveDateTime,
}
