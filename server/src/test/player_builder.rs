use bevy::prelude::*;
use bevy_nest::server::ClientId;
use fake::{faker::internet::en::Password, faker::name::en::Name, Dummy, Fake, Faker};
use sqlx::{types::Json, PgPool};

use crate::{
    auth::components::Authenticating,
    items::components::Inventory,
    player::{
        bundles::PlayerBundle,
        components::{Character, Client, Online},
        config::CharacterConfig,
    },
};

#[derive(Dummy)]
pub struct PlayerBuilder {
    id: i64,
    #[dummy(faker = "Name()")]
    name: String,
    #[dummy(faker = "Password(3..30)")]
    password: String,
    #[dummy(expr = "0")]
    role: i16,
    #[dummy(expr = "CharacterConfig::default()")]
    config: CharacterConfig,
    #[dummy(expr = "false")]
    authenticating: bool,
    #[dummy(expr = "false")]
    has_inventory: bool,
    #[dummy(expr = "None")]
    tile: Option<Entity>,
}

#[allow(dead_code)]
impl PlayerBuilder {
    pub fn new() -> Self {
        Faker.fake::<Self>()
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

    pub fn has_inventory(mut self, has_inventory: bool) -> Self {
        self.has_inventory = has_inventory;
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

    pub fn build(self, app: &mut App) -> (Entity, ClientId, Option<Entity>) {
        let client_id = ClientId::new();
        let mut inventory: Option<Entity> = None;

        let mut entity = app.world.spawn((Client {
            id: client_id,
            width: 80,
        },));

        if self.authenticating {
            entity.insert(Authenticating::default());
        } else {
            entity.insert((
                Online,
                PlayerBundle {
                    character: Character {
                        id: self.id,
                        name: self.name,
                        role: self.role,
                        config: self.config,
                    },
                },
            ));
        }

        if self.has_inventory {
            entity.with_children(|parent| {
                inventory = Some(parent.spawn(Inventory).id());
            });
        }

        if let Some(tile) = self.tile {
            entity.set_parent(tile);
        }

        (entity.id(), client_id, inventory)
    }
}
