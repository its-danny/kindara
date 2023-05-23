use crate::visual::components::Sprite;

use super::components::Tile;

pub(super) fn view_for_tile(tile: &Tile, sprite: &Sprite) -> String {
    format!("{} {}\n{}", sprite.character, tile.name, tile.description)
}
