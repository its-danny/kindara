use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::player::components::Character;

pub(super) fn who(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    players: Query<&Character>,
) {
    let regex = Regex::new(r"^who$").unwrap();

    for (message, _) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) if regex.is_match(text) => Some((message, text)),
        _ => None,
    }) {
        let online = players
            .iter()
            .map(|character| character.name.clone())
            .collect::<Vec<_>>();

        outbox.send_text(message.from, online.join(", "));
    }
}
