use bevy::prelude::*;
use fake::{
    faker::address::en::{BuildingNumber, CityName},
    faker::lorem::en::Sentence,
    Dummy, Fake, Faker,
};

use crate::{
    spatial::{
        bundles::TileBundle,
        components::{LifeSpawn, Position, Tile, Zone},
    },
    visual::components::Sprite,
};

#[derive(Dummy)]
pub struct ZoneBuilder {
    #[dummy(faker = "CityName()")]
    name: String,
}

impl ZoneBuilder {
    pub fn new() -> Self {
        Faker.fake::<Self>()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn build(self, app: &mut App) -> Entity {
        app.world.spawn(Zone { name: self.name }).id()
    }
}

#[derive(Dummy)]
pub struct TileBuilder {
    #[dummy(faker = "BuildingNumber()")]
    name: String,
    #[dummy(faker = "Sentence(1..3)")]
    description: String,
    #[dummy(expr = "IVec3::new(Faker.fake::<i32>(), Faker.fake::<i32>(), Faker.fake::<i32>())")]
    position: IVec3,
    #[dummy(expr = "false")]
    is_spawn: bool,
    #[dummy(expr = "\" \".into()")]
    sprite: String,
}

#[allow(dead_code)]
impl TileBuilder {
    pub fn new() -> Self {
        Faker.fake::<Self>()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = description.into();
        self
    }

    pub fn position(mut self, coords: IVec3) -> Self {
        self.position = coords;
        self
    }

    pub fn is_spawn(mut self) -> Self {
        self.is_spawn = true;
        self
    }

    pub fn sprite(mut self, sprite: &str) -> Self {
        self.sprite = sprite.into();
        self
    }

    pub fn build(self, app: &mut App, zone: Entity) -> Entity {
        let mut entity = app.world.spawn(TileBundle {
            tile: Tile {
                name: self.name,
                description: self.description,
            },
            position: Position(self.position),
            sprite: Sprite {
                character: self.sprite,
            },
        });

        entity.set_parent(zone);

        if self.is_spawn {
            entity.insert(LifeSpawn);
        }

        entity.id()
    }
}
