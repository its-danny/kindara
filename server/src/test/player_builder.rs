use bevy::prelude::*;
use bevy_nest::server::ClientId;
use sqlx::{types::Json, PgPool};

use crate::{
    auth::components::Authenticating,
    player::{
        bundles::PlayerBundle,
        components::{Character, Client},
        config::CharacterConfig,
    },
};

pub struct PlayerBuilder {
    id: i64,
    name: String,
    password: String,
    role: i16,
    config: CharacterConfig,
    authenticating: bool,
    tile: Option<Entity>,
}

impl PlayerBuilder {
    pub fn new() -> Self {
        Self {
            id: 0,
            name: "Anu".into(),
            password: bcrypt::hash("secret", bcrypt::DEFAULT_COST).unwrap(),
            role: 0,
            config: CharacterConfig::default(),
            authenticating: false,
            tile: None,
        }
    }

    pub fn id(mut self, id: i64) -> Self {
        self.id = id;
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn password(mut self, password: &str) -> Self {
        self.password = bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap();
        self
    }

    pub fn role(mut self, role: i16) -> Self {
        self.role = role;
        self
    }

    pub fn config(mut self, config: CharacterConfig) -> Self {
        self.config = config;
        self
    }

    pub fn authenticating(mut self, authenticating: bool) -> Self {
        self.authenticating = authenticating;
        self
    }

    pub fn tile(mut self, tile: Entity) -> Self {
        self.tile = Some(tile);
        self
    }

    pub async fn store(self, pool: &PgPool) -> Result<Self, sqlx::Error> {
        sqlx::query("INSERT INTO characters (id, name, password, config) VALUES ($1, $2, $3, $4)")
            .bind(&self.id)
            .bind(&self.name)
            .bind(&self.password)
            .bind(Json(self.config))
            .execute(pool)
            .await?;

        Ok(self)
    }

    pub fn build(self, app: &mut App) -> (ClientId, Entity) {
        let client_id = ClientId::new();

        let mut entity = app.world.spawn((Client {
            id: client_id,
            width: 80,
        },));

        if self.authenticating {
            entity.insert(Authenticating::default());
        } else {
            entity.insert(PlayerBundle {
                character: Character {
                    id: self.id,
                    name: self.name,
                    role: self.role,
                    config: self.config,
                },
            });
        }

        if let Some(tile) = self.tile {
            entity.set_parent(tile);
        }

        (client_id, entity.id())
    }
}
