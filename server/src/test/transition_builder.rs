use bevy::prelude::*;

use crate::spatial::components::{Position, Transition};

pub struct TransitionBuilder {
    tile: Entity,
    tags: Vec<String>,
    target: Entity,
}

impl TransitionBuilder {
    pub fn new(tile: Entity, target: Entity) -> Self {
        Self {
            tile,
            tags: vec![],
            target,
        }
    }

    pub fn tags(mut self, tags: &Vec<&str>) -> Self {
        self.tags = tags.iter().map(|t| t.to_string()).collect();
        self
    }

    pub fn build(self, app: &mut App) -> Entity {
        let position = app
            .world
            .get::<Position>(self.target)
            .expect("Target has no position");

        app.world
            .spawn(Transition {
                tags: self.tags,
                zone: position.zone,
                coords: position.coords,
            })
            .set_parent(self.tile)
            .id()
    }
}
