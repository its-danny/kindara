use bevy::prelude::*;

use crate::spatial::components::{Impassable, Position, Tile};

pub(super) fn create_world(mut commands: Commands) {
    commands.spawn((
        Position(IVec3::new(0, 0, 0)),
        Tile {
            name: "The Void".to_string(),
            description: "A vast, empty void.".to_string(),
        },
    ));

    commands.spawn((
        Position(IVec3::new(0, 1, 0)),
        Tile {
            name: "More Void".to_string(),
            description: "A vast, empty void.".to_string(),
        },
    ));

    commands.spawn((
        Impassable,
        Position(IVec3::new(0, 2, 0)),
        Tile {
            name: "Even More Void".to_string(),
            description: "A vast, empty void.".to_string(),
        },
    ));

    commands.spawn((
        Position(IVec3::new(0, 0, 1)),
        Tile {
            name: "Upper Void".to_string(),
            description: "A vast, empty void.".to_string(),
        },
    ));
}
