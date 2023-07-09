use bevy::prelude::*;

#[derive(Component, Reflect, FromReflect, Clone)]
pub struct Attributes {
    pub vitality: u32,
    pub proficiency: u32,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            vitality: 10,
            proficiency: 5,
        }
    }
}

impl Attributes {
    pub fn max_health(&self) -> u32 {
        self.vitality * 10
    }

    #[allow(dead_code)]
    pub fn base_damage(&self) -> u32 {
        self.proficiency * 2
    }
}

#[derive(Component)]
pub struct State {
    pub health: u32,
}
