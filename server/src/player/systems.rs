use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::net::telnet::NAWS;

use super::components::Client;

pub fn handle_client_width(mut inbox: EventReader<Inbox>, mut clients: Query<&mut Client>) {
    for (message, content) in inbox.iter().filter_map(|m| {
        if let Message::Command(content) = &m.content {
            Some((m, content))
        } else {
            None
        }
    }) {
        let Some(mut client) = clients.iter_mut().find(|c| c.id == message.from) else {
            debug!("Could not find player for client: {:?}", message.from);

            continue;
        };

        if content[0..=2] == [IAC, SB, NAWS] {
            let width = content.get(4).map(|a| *a as u16).unwrap_or(80);

            if width > 0 {
                client.width = width;
            }
        }
    }
}
