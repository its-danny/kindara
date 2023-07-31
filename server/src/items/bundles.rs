use bevy::prelude::*;
use bevy_proto::prelude::*;

use crate::visual::components::Depiction;

use super::components::Item;

#[derive(Bundle, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct ItemBundle {
    pub item: Item,
    pub depiction: Depiction,
}
