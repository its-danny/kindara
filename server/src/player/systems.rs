use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{command_messages, net::telnet::NAWS};

use super::components::Client;

pub fn handle_client_width(mut inbox: EventReader<Inbox>, mut clients: Query<&mut Client>) {
    for (message, content) in command_messages!(inbox) {
        let Some(mut client) = clients.iter_mut().find(|c| c.id == message.from) else {
            return;
        };

        if content[0..=2] == [IAC, SB, NAWS] {
            let width = content.get(4).map(|a| *a as u16).unwrap_or(80);

            if width > 0 {
                client.width = width;
            }
        }
    }
}
