use bevy::prelude::*;
use fake::{Dummy, Fake, Faker};

use crate::{
    interact::components::{Interaction, Interactions},
    npc::{bundles::NpcBundle, components::Npc},
    visual::components::Depiction,
};

#[derive(Dummy)]
pub struct NpcBuilder {
    name: String,
    short_name: String,
    description: String,
    tags: Vec<String>,
    #[dummy(expr = "None")]
    interactions: Option<Vec<Interaction>>,
    #[dummy(expr = "None")]
    tile: Option<Entity>,
}

#[allow(dead_code)]
impl NpcBuilder {
    pub fn new() -> Self {
        Faker.fake::<Self>()
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn short_name(mut self, short_name: &str) -> Self {
        self.short_name = short_name.to_string();
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

    pub fn interactions(mut self, interactions: Vec<Interaction>) -> Self {
        self.interactions = Some(interactions);
        self
    }

    pub fn tile(mut self, tile: Entity) -> Self {
        self.tile = Some(tile);
        self
    }

    pub fn build(self, app: &mut App) -> Entity {
        let mut entity = app.world.spawn(NpcBundle {
            npc: Npc,
            depiction: Depiction {
                name: self.name,
                short_name: self.short_name,
                description: self.description,
                tags: self.tags,
                visible: true,
            },
        });

        if let Some(tile) = self.tile {
            entity.set_parent(tile);
        }

        if let Some(interactions) = self.interactions {
            entity.insert(Interactions(interactions));
        }

        entity.id()
    }
}
