use chrono::NaiveDateTime;
use sqlx::{types::Json, FromRow};

use crate::player::config::CharacterConfig;

#[derive(FromRow)]
pub struct CharacterModel {
    pub id: i64,
    pub name: String,
    pub password: String,
    pub email: Option<String>,
    pub role: i16,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub config: Json<CharacterConfig>,
}
