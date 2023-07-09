use bevy_nest::prelude::*;

pub struct Prompt {
    pub client_id: ClientId,
}

impl Prompt {
    pub fn new(client_id: ClientId) -> Self {
        Self { client_id }
    }
}
