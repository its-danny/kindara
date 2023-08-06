use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{ChatChannel, Command, ParseError, ParsedCommand},
    paint,
    player::components::{Character, Client, Online},
    value_or_continue,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_chat(content: &str) -> Result<Command, ParseError> {
    let regex =
        REGEX.get_or_init(|| Regex::new(r"^(?P<channel>chat|novice)( (?P<message>.*))?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let channel = captures
                .name("channel")
                .map(|m| m.as_str().trim())
                .unwrap_or("chat");

            let channel = match channel {
                "chat" | "c" => ChatChannel::Chat,
                "novice" | "n" => ChatChannel::Novice,
                _ => ChatChannel::Chat,
            };

            let message = captures
                .name("message")
                .map(|m| m.as_str().trim())
                .filter(|m| !m.is_empty())
                .ok_or(ParseError::InvalidArguments("Say what?".into()))?;

            Ok(Command::Chat((channel, message.into())))
        }
    }
}

pub fn chat(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character), With<Online>>,
) {
    for command in commands.iter() {
        if let Command::Chat((channel, message)) = &command.command {
            let (_, character) =
                value_or_continue!(players.iter().find(|(c, _)| c.id == command.from));

            for (client, other_character) in players.iter() {
                let mentioned = message
                    .to_lowercase()
                    .contains(&other_character.name.to_lowercase());

                let message = if mentioned {
                    format!("<fg.yellow>{message}</>")
                } else {
                    message.clone()
                };

                outbox.send_text(
                    client.id,
                    paint!(
                        "[<fg.{}>{}</>] {}: {message}",
                        channel.color(),
                        channel,
                        character.name
                    ),
                );
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
        tile_builder::{TileBuilder, ZoneBuilder},
        utils::{get_message_content, send_message},
    };

    #[test]
    fn parses() {
        let message = handle_chat("chat Hello!");
        assert_eq!(
            message,
            Ok(Command::Chat((ChatChannel::Chat, "Hello!".into())))
        );

        let no_message = handle_chat("chat");
        assert_eq!(
            no_message,
            Err(ParseError::InvalidArguments("Say what?".into()))
        );
    }

    #[test]
    fn sends_to_sender() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, chat);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .name("Astrid")
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "chat Hello!");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, format!("[chat] Astrid: Hello!"));
    }

    #[test]
    fn sends_to_everyone() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, chat);

        let zone_one = ZoneBuilder::new().build(&mut app);
        let tile_one = TileBuilder::new().build(&mut app, zone_one);

        let zone_two = ZoneBuilder::new().build(&mut app);
        let tile_two = TileBuilder::new().build(&mut app, zone_two);

        let (_, sender_client_id, _) = PlayerBuilder::new()
            .tile(tile_one)
            .name("Flora")
            .build(&mut app);

        let (_, recipient_client_id, _) = PlayerBuilder::new().tile(tile_two).build(&mut app);

        send_message(&mut app, sender_client_id, "chat Hello!");
        app.update();

        let content = get_message_content(&mut app, recipient_client_id).unwrap();

        assert_eq!(content, "[chat] Flora: Hello!");
    }

    #[test]
    fn empty_message() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, chat);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "chat   ");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Say what?");
    }
}
