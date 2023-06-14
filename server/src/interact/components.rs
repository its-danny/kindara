use std::fmt::{Display, Formatter};

use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Interactions(pub Vec<Interaction>);

#[derive(PartialEq, Reflect, FromReflect, Debug)]
pub enum Interaction {
    Take,
    Place,
}

impl Display for Interaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Interaction::Take => write!(f, "Take"),
            Interaction::Place => write!(f, "Place"),
        }
    }
}

impl Interaction {
    pub fn usable_in_menu(&self) -> bool {
        match self {
            Interaction::Take => true,
            Interaction::Place => false,
        }
    }
}

#[derive(Component)]
pub struct InMenu(pub MenuType);

pub enum MenuType {
    Examine(Entity),
}
