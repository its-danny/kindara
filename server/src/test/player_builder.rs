use bevy::{ecs::world::EntityMut, prelude::*};
use bevy_nest::server::ClientId;
use sqlx::{types::Json, PgPool};

use crate::{
    auth::components::Authenticating,
    player::{
        bundles::PlayerBundle,
        components::{Character, Client},
        config::CharacterConfig,
    },
    spatial::components::{Position, Zone},
};

pub struct PlayerBuilder {
    id: i64,
    name: String,
    password: String,
    role: i16,
    config: CharacterConfig,
    zone: Zone,
    coords: IVec3,
    authenticating: bool,
}

impl PlayerBuilder {
    pub fn new() -> Self {
        Self {
            id: 0,
            name: "Anu".into(),
            password: bcrypt::hash("secret", bcrypt::DEFAULT_COST).unwrap(),
            role: 0,
            config: CharacterConfig::default(),
            zone: Zone::Void,
            coords: IVec3::ZERO,
            authenticating: false,
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

    pub fn zone(mut self, zone: Zone) -> Self {
        self.zone = zone;
        self
    }

    pub fn coords(mut self, coords: IVec3) -> Self {
        self.coords = coords;
        self
    }

    pub fn authenticating(mut self, authenticating: bool) -> Self {
        self.authenticating = authenticating;
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
            width: u16::MAX,
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
                position: Position {
                    zone: self.zone,
                    coords: self.coords,
                },
            });
        }

        (client_id, entity.id())
    }
}
