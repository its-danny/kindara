use std::fmt::{Display, Formatter};

use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Schematic, Reflect)]
#[reflect(Schematic)]
pub struct Interactions(pub Vec<Interaction>);

#[derive(PartialEq, Reflect, Debug)]
pub enum Interaction {
    Attack,
    Place,
    Take,
}

impl Display for Interaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Interaction::Attack => write!(f, "Attack"),
            Interaction::Place => write!(f, "Place"),
            Interaction::Take => write!(f, "Take"),
        }
    }
}

impl Interaction {
    pub fn usable_in_menu(&self) -> bool {
        match self {
            Interaction::Attack => true,
            Interaction::Place => false,
            Interaction::Take => true,
        }
    }
}

#[derive(Component)]
pub struct InMenu(pub MenuType);

pub enum MenuType {
    Examine(Entity),
}
