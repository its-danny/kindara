use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{auth::components::Authenticating, player::components::Client};

use super::telnet::NAWS;

pub fn on_network_event(
    mut bevy: Commands,
    mut events: EventReader<NetworkEvent>,
    mut outbox: EventWriter<Outbox>,
    clients: Query<(Entity, &Client)>,
) {
    for event in events.iter() {
        if let NetworkEvent::Connected(id) = event {
            bevy.spawn((Client { id: *id, width: 80 }, Authenticating::default()));

            outbox.send_command(*id, vec![IAC, WILL, GMCP]);
            outbox.send_command(*id, vec![IAC, DO, NAWS]);

            outbox.send_text(
                *id,
                "Thou hast arrived in Aureus, wanderer. What name dost thou bear?",
            );
        }

        if let NetworkEvent::Disconnected(id) = event {
            if let Some((entity, _)) = clients.iter().find(|(_, c)| c.id == *id) {
                bevy.entity(entity).despawn();
            }
        }
    }
}
