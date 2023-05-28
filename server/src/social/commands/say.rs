use bevy::prelude::*;
use bevy_nest::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
    spatial::components::Position,
};

static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^((say |')(?P<message>.+))$").unwrap());

pub fn parse_say(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if let Some(captures) = REGEX.captures(content) {
        let message = captures.name("message").map(|m| m.as_str()).unwrap_or("");

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Say(message.to_string()),
        });

        true
    } else {
        false
    }
}

pub fn say(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position, &Character)>,
) {
    for command in commands.iter() {
        if let Command::Say(message) = &command.command {
            let Some((client, position, character)) = players.iter().find(|(c, _, _)| c.id == command.from) else {
                return;
            };

            let message = message.trim();

            if message.is_empty() {
                outbox.send_text(client.id, "Say what?");

                return;
            }

            outbox.send_text(client.id, format!("You say \"{message}\""));

            for (other_client, other_position, _) in
                players.iter().filter(|(c, _, _)| c.id != client.id)
            {
                if position.zone == other_position.zone && position.coords == other_position.coords
                {
                    outbox.send_text(
                        other_client.id,
                        format!("{} says \"{message}\"", character.name),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        spatial::components::Zone,
        test::{app_builder::AppBuilder, player_builder::PlayerBuilder, tile_builder::TileBuilder},
        world::resources::TileMap,
    };

    #[test]
    fn test_say() {
        let mut app = AppBuilder::new();
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
