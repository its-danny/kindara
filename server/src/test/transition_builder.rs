use bevy::prelude::*;

use crate::spatial::{
    bundles::TransitionBundle,
    components::{Position, Transition, Zone},
};

pub struct TransitionBuilder {
    tags: Vec<String>,
    target_zone: Zone,
    target_coords: IVec3,
    zone: Zone,
    coords: IVec3,
}

impl TransitionBuilder {
    pub fn new() -> Self {
        Self {
            tags: vec![],
            target_zone: Zone::Void,
            target_coords: IVec3::ZERO,
            zone: Zone::Void,
            coords: IVec3::ZERO,
        }
    }

    pub fn tags(mut self, tags: &Vec<&str>) -> Self {
        self.tags = tags.iter().map(|t| t.to_string()).collect();
        self
    }

    pub fn target_zone(mut self, zone: Zone) -> Self {
        self.target_zone = zone;
        self
    }

    pub fn target_coords(mut self, coords: IVec3) -> Self {
        self.target_coords = coords;
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

    pub fn build(self, app: &mut App) -> Entity {
        app.world
            .spawn(TransitionBundle {
                transition: Transition {
                    tags: self.tags,
                    zone: self.target_zone,
                    coords: self.target_coords,
                },
                position: Position {
                    zone: self.zone,
                    coords: self.coords,
                },
            })
            .id()
    }
}
