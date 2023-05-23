use bevy::prelude::*;

use crate::visual::components::Sprite;

use super::components::Tile;

pub(super) fn offset_for_direction(direction: &str) -> Option<IVec3> {
    match direction {
        "north" | "n" => Some(IVec3::new(0, -1, 0)),
        "northeast" | "ne" => Some(IVec3::new(1, -1, 0)),
        "east" | "e" => Some(IVec3::new(1, 0, 0)),
        "southeast" | "se" => Some(IVec3::new(1, 1, 0)),
        "south" | "s" => Some(IVec3::new(0, 1, 0)),
        "southwest" | "sw" => Some(IVec3::new(-1, 1, 0)),
        "west" | "w" => Some(IVec3::new(-1, 0, 0)),
        "northwest" | "nw" => Some(IVec3::new(-1, -1, 0)),
        "up" | "u" => Some(IVec3::new(0, 0, 1)),
        "down" | "d" => Some(IVec3::new(0, 0, -1)),
        _ => None,
    }
}

pub(super) fn view_for_tile(tile: &Tile, sprite: &Sprite) -> String {
    format!("{} {}\n{}", sprite.character, tile.name, tile.description)
}
