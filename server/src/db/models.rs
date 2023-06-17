use chrono::NaiveDateTime;
use sqlx::{types::Json, FromRow};

use crate::{player::config::CharacterConfig, world::resources::WorldState};

#[derive(sqlx::Type)]
#[sqlx(type_name = "character_role", rename_all = "lowercase")]
pub enum Role {
    Admin,
    Player,
}

#[derive(FromRow)]
pub struct CharacterModel {
    pub config: Json<CharacterConfig>,
    pub description: Option<String>,
    pub email: Option<String>,
    pub id: i64,
    pub name: String,
    pub password: String,
    pub role: Role,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, FromRow)]
pub struct WorldSaveModel {
    pub id: i64,
    pub state: Json<WorldState>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
