use bevy::prelude::*;
use bevy_nest::server::ClientId;

#[derive(Component)]
pub struct Client(pub ClientId);

#[derive(Component)]
pub struct Character {
    pub name: String,
    pub role: i16,
}

impl Character {
    pub fn can(&self, permission: i16) -> bool {
        self.role & permission == permission
    }
}

#[cfg(test)]
mod tests {
    use crate::player::permissions::*;

    use super::*;

    #[test]
    fn test_permissions() {
        let admin = Character {
            name: "admin".to_string(),
            role: TELEPORT,
        };

        assert!(admin.can(TELEPORT));

        let player = Character {
            name: "player".to_string(),
            role: 0,
        };

        assert!(!player.can(TELEPORT));
    }
}
