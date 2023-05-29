use bevy::prelude::*;

use crate::{
    spatial::{
        bundles::TileBundle,
        components::{Impassable, Position, Tile, Zone},
    },
    visual::components::Sprite,
    world::resources::TileMap,
};

pub struct TileBuilder {
    name: String,
    description: String,
    zone: Zone,
    coords: IVec3,
    sprite: String,
    impassable: bool,
}

impl TileBuilder {
    pub fn new() -> Self {
        Self {
            name: "The Void".into(),
            description: "A vast, empty void.".into(),
            zone: Zone::Void,
            coords: IVec3::ZERO,
            sprite: "x".into(),
            impassable: false,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn description(mut self, description: &str) -> Self {
        self.description = description.into();
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

    pub fn sprite(mut self, sprite: &str) -> Self {
        self.sprite = sprite.into();
        self
    }

    pub fn impassable(mut self, impassable: bool) -> Self {
        self.impassable = impassable;
        self
    }

    pub fn build(self, app: &mut App) -> Entity {
        let entity = app
            .world
            .spawn(TileBundle {
                tile: Tile {
                    name: self.name,
                    description: self.description,
                },
                position: Position {
                    zone: self.zone,
                    coords: self.coords,
                },
                sprite: Sprite {
                    character: self.sprite,
                },
            })
            .id();

        if self.impassable {
            app.world.entity_mut(entity).insert(Impassable);
        }

        app.world
            .resource_mut::<TileMap>()
            .insert((self.zone, self.coords), entity);

        entity
    }
}
