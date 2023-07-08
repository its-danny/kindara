use bevy_nest::server::ClientId;

pub struct Prompt {
    pub client_id: ClientId,
}

impl Prompt {
    pub fn new(client_id: ClientId) -> Self {
        Self { client_id }
    }
}
