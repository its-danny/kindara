use bevy::prelude::*;
use bevy_proto::prelude::*;

use crate::spatial::components::{Position, Tile};

use super::resources::TileMap;

pub fn spawn_void(mut commands: ProtoCommands) {
    commands.spawn("world.void");
}

pub fn spawn_testing_movement(mut commands: ProtoCommands) {
    commands.spawn("world.testing.movement");
}

pub fn on_tile_added(
    mut tile_map: ResMut<TileMap>,
    tiles: Query<(Entity, &Position), Added<Tile>>,
) {
    for (entity, position) in tiles.iter() {
        tile_map.insert((position.zone, position.coords), entity);
    }
}

pub fn on_tile_removed(
    mut tile_map: ResMut<TileMap>,
    tiles: Query<(Entity, &Position)>,
    mut removals: RemovedComponents<Tile>,
) {
    for entity in removals.iter() {
        if let Ok((_, position)) = tiles.get(entity) {
            tile_map.remove((position.zone, position.coords));
        }
    }
}
