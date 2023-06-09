use bevy::prelude::*;
use fake::{Dummy, Fake, Faker};

use crate::items::components::Item;

#[derive(Dummy)]
pub struct ItemBuilder {
    name: String,
    name_on_ground: String,
    description: String,
    tags: Vec<String>,
    #[dummy(expr = "None")]
    tile: Option<Entity>,
}

#[allow(dead_code)]
impl ItemBuilder {
    pub fn new() -> Self {
        Faker.fake::<Self>()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn name_on_ground(mut self, name_on_ground: &str) -> Self {
        self.name_on_ground = name_on_ground.to_string();
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    pub fn tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.iter().map(|t| t.to_string()).collect();
        self
    }

    pub fn tile(mut self, tile: Entity) -> Self {
        self.tile = Some(tile);
        self
    }

    pub fn build(self, app: &mut App) -> Entity {
        let mut entity = app.world.spawn(Item {
            name: self.name,
            name_on_ground: self.name_on_ground,
            description: self.description,
            tags: self.tags,
        });

        if let Some(tile) = self.tile {
            entity.set_parent(tile);
        }

        entity.id()
    }
}
