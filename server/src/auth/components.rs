use bevy::prelude::*;

#[derive(Component)]
pub(super) struct Authenticating {
    pub(super) state: AuthState,
    pub(super) name: String,
}

impl Default for Authenticating {
    fn default() -> Self {
        Self {
            state: AuthState::AwaitingName,
            name: "".to_string(),
        }
    }
}

#[derive(PartialEq, Eq)]
pub(super) enum AuthState {
    AwaitingName,
    AwaitingPassword,
}
