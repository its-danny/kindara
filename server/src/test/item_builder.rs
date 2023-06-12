use bevy::prelude::*;
use fake::{Dummy, Fake, Faker};

use crate::{
    interact::components::{Interaction, Interactions},
    items::components::{Item, Size, Surface, SurfaceKind},
};

#[derive(Dummy)]
pub struct ItemBuilder {
    name: String,
    short_name: String,
    description: String,
    tags: Vec<String>,
    #[dummy(expr = "None")]
    interactions: Option<Vec<Interaction>>,
    #[dummy(expr = "false")]
    is_surface: bool,
    #[dummy(expr = "None")]
    surface_kind: Option<SurfaceKind>,
    #[dummy(expr = "None")]
    surface_capacity: Option<u8>,
    #[dummy(expr = "Size::Small")]
    size: Size,
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

    pub fn is_surface(mut self, kind: SurfaceKind, capacity: u8) -> Self {
        self.is_surface = true;
        self.surface_kind = Some(kind);
        self.surface_capacity = Some(capacity);
        self
    }

    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    pub fn tile(mut self, tile: Entity) -> Self {
        self.tile = Some(tile);
        self
    }

    pub fn build(self, app: &mut App) -> Entity {
        let mut entity = app.world.spawn(Item {
            name: self.name,
            short_name: self.short_name,
            description: self.description,
            tags: self.tags,
            size: self.size,
            visible: true,
        });

        if let Some(tile) = self.tile {
            entity.set_parent(tile);
        }

        if let Some(interactions) = self.interactions {
            entity.insert(Interactions(interactions));
        }

        if self.is_surface {
            entity.insert(Surface {
                kind: self.surface_kind.unwrap(),
                capacity: self.surface_capacity.unwrap(),
            });
        }

        entity.id()
    }
}
