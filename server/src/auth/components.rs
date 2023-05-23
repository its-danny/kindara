use bevy::prelude::*;

#[derive(Component)]
pub(super) struct Authenticating {
    pub(super) state: AuthState,
    pub(super) name: String,
}

impl Default for Authenticating {
    fn default() -> Self {
        Self {
            state: AuthState::Name,
            name: "".to_string(),
        }
    }
}

#[derive(PartialEq, Eq)]
pub(super) enum AuthState {
    /// Waiting for the client to send their name.
    Name,
    /// Waiting for the client to send their password.
    Password,
    /// We want to ignore any messages until the current async task is complete.
    AwaitingTaskCompletion,
}
