use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Item {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub visible: bool,
}

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct CanTake;

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct CanPlace;

#[derive(Component)]
pub struct Inventory;

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Surface {
    pub kind: SurfaceType,
    pub limit: u8,
}

#[derive(Component, Reflect, FromReflect)]
pub enum SurfaceType {
    Floor,
    Wall,
    Ceiling,
    Interior,
}

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct PlacementSize(Size);

#[derive(Component, Reflect, FromReflect)]
pub enum Size {
    Small,
    Medium,
    Large,
}

#[allow(dead_code)]
impl Size {
    const fn value(self) -> u8 {
        match self {
            Self::Small => 1,
            Self::Medium => 3,
            Self::Large => 5,
        }
    }
}
