use bevy::prelude::*;

pub const SHUTDOWN: u32 = 1 << 2;
pub const ANNOUNCE: u32 = 1 << 3;
pub const TELEPORT: u32 = 1 << 4;

const PLAYER: u32 = 0;
const ADMIN: u32 = PLAYER | SHUTDOWN | ANNOUNCE | TELEPORT;

#[derive(Component)]
pub struct Keycard {
    permissions: u32,
}

impl Keycard {
    pub fn admin() -> Self {
        Self { permissions: ADMIN }
    }

    pub fn player() -> Self {
        Self {
            permissions: PLAYER,
        }
    }

    pub fn can(&self, permission: u32) -> bool {
        (self.permissions & permission) == permission
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admin_keycard() {
        let keycard = Keycard::admin();

        assert!(keycard.can(SHUTDOWN));
        assert!(keycard.can(ANNOUNCE));
        assert!(keycard.can(TELEPORT));
    }

    #[test]
    fn player_keycard() {
        let keycard = Keycard::player();

        assert!(!keycard.can(SHUTDOWN));
        assert!(!keycard.can(ANNOUNCE));
        assert!(!keycard.can(TELEPORT));
    }
}
