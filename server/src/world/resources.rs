use bevy::{prelude::*, utils::HashMap};

use crate::spatial::components::Zone;

#[derive(Hash, Eq, PartialEq)]
pub struct Key(Zone, IVec3);

#[derive(Default, Resource)]
pub struct TileMap(pub HashMap<Key, Entity>);

impl TileMap {
    pub fn insert(&mut self, key: (Zone, IVec3), entity: Entity) {
        self.0.insert(Key(key.0, key.1), entity);
    }

    pub fn get(&self, zone: Zone, coords: IVec3) -> Option<&Entity> {
        self.0.get(&Key(zone, coords))
    }

    pub fn remove(&mut self, key: (Zone, IVec3)) {
        self.0.remove(&Key(key.0, key.1));
    }
}
