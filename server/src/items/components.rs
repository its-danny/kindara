use std::fmt::{self, Display, Formatter};

use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component)]
pub struct Inventory;

#[derive(Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct Item {
    pub size: Size,
}

#[derive(Copy, Clone, Reflect)]
pub enum Size {
    Small,
    Medium,
    Large,
}

impl Size {
    pub const fn value(self) -> u8 {
        match self {
            Self::Small => 1,
            Self::Medium => 3,
            Self::Large => 5,
        }
    }
}

#[derive(Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct Surface {
    /// The kind of surface this is.
    pub kind: SurfaceKind,
    /// The maximum number of items that can be placed on this surface
    /// as defined by their size.
    pub capacity: u8,
}

#[derive(Reflect)]
pub enum SurfaceKind {
    Floor,
    Wall,
    Ceiling,
    Interior,
}

impl Display for SurfaceKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Floor => write!(f, "on"),
            Self::Wall => write!(f, "against"),
            Self::Ceiling => write!(f, "on"),
            Self::Interior => write!(f, "in"),
        }
    }
}
