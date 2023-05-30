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
    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        tile_builder::TileBuilder,
        utils::{get_message_content, send_message},
    };

    #[test]
    fn sends_to_sender() {
        let mut app = AppBuilder::new().build();
        app.add_system(say);

        TileBuilder::new().build(&mut app);

        let (client_id, _) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "say Hello!");

        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You say \"Hello!\"");
    }

    #[test]
    fn sends_to_tile() {
        let mut app = AppBuilder::new().build();
        app.add_system(say);

        TileBuilder::new().build(&mut app);

        let (sender_client_id, _) = PlayerBuilder::new().name("Flora").build(&mut app);
        let (recipient_client_id, _) = PlayerBuilder::new().name("Salus").build(&mut app);

        send_message(&mut app, sender_client_id, "say Hello!");

        app.update();

        let content = get_message_content(&mut app, recipient_client_id);

        assert_eq!(content, "Flora says \"Hello!\"");
    }

    #[test]
    fn empty_message() {
        let mut app = AppBuilder::new().build();
        app.add_system(say);

        TileBuilder::new().build(&mut app);

        let (client_id, _) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "say   ");

        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Say what?");
    }
}
