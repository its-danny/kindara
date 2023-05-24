use bevy::prelude::*;

#[derive(Component, Reflect, FromReflect)]
pub struct Sprite {
    pub character: String,
}
