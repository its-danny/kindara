use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{net::telnet::NAWS, value_or_continue};

use super::components::Client;

pub fn handle_client_width(mut inbox: EventReader<Inbox>, mut clients: Query<&mut Client>) {
    for (message, content) in inbox.iter().filter_map(|m| {
        if let Message::Command(content) = &m.content {
            Some((m, content))
        } else {
            None
        }
    }) {
        let mut client = value_or_continue!(clients.iter_mut().find(|c| c.id == message.from));

        if content[0..=2] == [IAC, SB, NAWS] {
            let width = content.get(4).map(|a| *a as u16).unwrap_or(80);

            if width > 0 {
                client.width = width;
            }
        }
    }
}
