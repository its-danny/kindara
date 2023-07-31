use bevy::prelude::*;
use bevy_nest::prelude::*;

#[derive(Event)]
pub struct Prompt {
    pub client_id: ClientId,
}

impl Prompt {
    pub fn new(client_id: ClientId) -> Self {
        Self { client_id }
    }
}
