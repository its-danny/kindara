use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::player::components::{Character, Client};

pub fn who(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character)>,
) {
    let regex = Regex::new(r"^who$").unwrap();

    for (message, _) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) if regex.is_match(text) => Some((message, text)),
        _ => None,
    }) {
        let Some((client, _)) = players.iter().find(|(c, _)| c.id == message.from) else {
            return;
        };

        let online = players
            .iter()
            .map(|(_, character)| character.name.clone())
            .collect::<Vec<_>>();

        outbox.send_text(client.id, online.join(", "));
    }
}
