use bevy::prelude::*;
use bevy_nest::server::ClientId;

use crate::{
    player::{
        bundles::PlayerBundle,
        components::{Character, Client},
    },
    spatial::components::{Position, Zone},
};

pub struct PlayerBuilder {
    name: String,
    role: i16,
    zone: Zone,
    coords: IVec3,
}

impl PlayerBuilder {
    pub fn new() -> Self {
        Self {
            name: "Ramose".into(),
            role: 0,
            zone: Zone::Void,
            coords: IVec3::ZERO,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.into();
        self
    }

    pub fn role(mut self, role: i16) -> Self {
        self.role = role;
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

    pub fn build(self, app: &mut App) -> (ClientId, Entity) {
        let client_id = ClientId::new();

        let entity = app
            .world
            .spawn((
                Client(client_id),
                PlayerBundle {
                    character: Character {
                        name: self.name,
                        role: self.role,
                    },
                    position: Position {
                        zone: self.zone,
                        coords: self.coords,
                    },
                },
            ))
            .id();

        (client_id, entity)
    }
}
