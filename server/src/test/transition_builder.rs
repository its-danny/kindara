use bevy::prelude::*;
use fake::{Dummy, Fake, Faker};

use crate::spatial::components::{Position, Transition, Zone};

#[derive(Dummy)]
pub struct TransitionBuilder {
    tags: Vec<String>,
}

#[allow(dead_code)]
impl TransitionBuilder {
    pub fn new() -> Self {
        Faker.fake::<Self>()
    }

    pub fn tags(mut self, tags: &Vec<&str>) -> Self {
        self.tags = tags.iter().map(|t| t.to_string()).collect();
        self
    }

    pub fn build(self, app: &mut App, tile: Entity, target: Entity) -> Entity {
        let target_parent = app.world.get::<Parent>(target).expect("Tile has no parent");

        let zone = app
            .world
            .get::<Zone>(target_parent.get())
            .expect("Target parent has no zone");

        let position = app
            .world
            .get::<Position>(target)
            .expect("Target has no position");

        app.world
            .spawn(Transition {
                tags: self.tags,
                zone: zone.name.clone(),
                position: position.0,
            })
            .set_parent(tile)
            .id()
    }
}
