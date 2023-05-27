use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::net::telnet::NAWS;

use super::components::Client;

pub fn handle_client_width(mut inbox: EventReader<Inbox>, mut clients: Query<&mut Client>) {
    for message in inbox
        .iter()
        .filter(|m| matches!(m.content, Message::Command(_)))
    {
        let Some(mut client) = clients.iter_mut().find(|c| c.id == message.from) else {
            return;
        };

        if let Message::Command(command) = &message.content {
            if command[0..=2] == [IAC, SB, NAWS] {
                let width = command.get(4).map(|a| *a as u16).unwrap_or(80);

                if width > 0 {
                    client.width = width;
                }
            }
        }
    }
}
