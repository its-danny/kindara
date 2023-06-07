use bevy::prelude::*;

use crate::{
    spatial::{
        bundles::TileBundle,
        components::{Position, Spawn, Tile, Zone},
    },
    visual::components::Sprite,
};

pub struct TileBuilder {
    name: String,
    description: String,
    zone: Zone,
    coords: IVec3,
    is_spawn: bool,
    sprite: String,
}

impl TileBuilder {
    pub fn new() -> Self {
        Self {
            name: "The Void".into(),
            description: "A vast, empty void.".into(),
            zone: Zone::Void,
            coords: IVec3::ZERO,
            is_spawn: false,
            sprite: "x".into(),
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

    pub fn is_spawn(mut self) -> Self {
        self.is_spawn = true;
        self
    }

    pub fn sprite(mut self, sprite: &str) -> Self {
        self.sprite = sprite.into();
        self
    }

    pub fn build(self, app: &mut App) -> Entity {
        let mut entity = app.world.spawn(TileBundle {
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
        });

        if self.is_spawn {
            entity.insert(Spawn);
        }

        entity.id()
    }
}
