use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    player::components::{Character, Client},
    spatial::components::Position,
};

pub fn say(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position, &Character)>,
) {
    let regex = Regex::new(r"^((say |')(?P<message>.+))$").unwrap();

    for (message, captures) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) => regex.captures(text).map(|caps| (message, caps)),
        _ => None,
    }) {
        let Some((client, position, character)) = players.iter().find(|(c, _, _)| c.id == message.from) else {
            return;
        };

        let message = captures
            .name("message")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();

        if message.is_empty() {
            outbox.send_text(client.id, "Say what?");

            return;
        }

        outbox.send_text(client.id, format!("You say \"{message}\""));

        for (other_client, other_position, _) in
            players.iter().filter(|(c, _, _)| c.id != client.id)
        {
            if position.zone == other_position.zone && position.coords == other_position.coords {
                outbox.send_text(
                    other_client.id,
                    format!("{} says \"{message}\"", character.name),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        spatial::components::Zone,
        test::{player_builder::PlayerBuilder, tile_builder::TileBuilder},
        world::resources::TileMap,
    };

    #[test]
    fn test_say() {
        let mut app = App::new();

        app.insert_resource(TileMap::default());
        app.add_event::<Inbox>();
        app.add_event::<Outbox>();
        app.add_system(say);

        let tile = TileBuilder::new().build(&mut app);

        app.world
            .resource_mut::<TileMap>()
            .insert((Zone::Void, IVec3::ZERO), tile);

        let (sender_client_id, _) = PlayerBuilder::new().name("Morrigan").build(&mut app);
        let (recipient_client_id, _) = PlayerBuilder::new().name("Astrid").build(&mut app);

        app.world.resource_mut::<Events<Inbox>>().send(Inbox {
            from: sender_client_id,
            content: Message::Text("say Hello!".into()),
        });

        app.update();

        let outbox_events = app.world.resource::<Events<Outbox>>();
        let mut outbox_reader = outbox_events.get_reader();

        let sender_response = outbox_reader
            .iter(outbox_events)
            .find(|r| r.to == sender_client_id)
            .expect("Expected response");

        assert_eq!(sender_response.to, sender_client_id);

        let content = match &sender_response.content {
            Message::Text(text) => text,
            _ => panic!("Expected text message"),
        };

        assert_eq!(content, "You say \"Hello!\"");

        let recipient_response = outbox_reader
            .iter(outbox_events)
            .find(|r| r.to == recipient_client_id)
            .expect("Expected response");

        assert_eq!(recipient_response.to, recipient_client_id);

        let content = match &recipient_response.content {
            Message::Text(text) => text,
            _ => panic!("Expected text message"),
        };

        assert_eq!(content, "Morrigan says \"Hello!\"");
    }
}
