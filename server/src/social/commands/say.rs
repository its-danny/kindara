use std::sync::OnceLock;

use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Character, Client, Online},
    spatial::components::Tile,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_say(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(say( |$)|' ?)(?P<message>.*)?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let message = captures
                .name("message")
                .map(|m| m.as_str().trim())
                .filter(|m| !m.is_empty())
                .ok_or(ParseError::InvalidArguments("Say what?".into()))?;

            Ok(Command::Say(message.into()))
        }
    }
}

#[sysfail(log)]
pub fn say(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Say(message) = &command.command {
            let (_, character, tile) = players
                .iter()
                .find(|(c, _, _)| c.id == command.from)
                .context("Player not found")?;

            let siblings = tiles.get(tile.get())?;

            for (other_client, _, _) in siblings.iter().filter_map(|c| players.get(*c).ok()) {
                outbox.send_text(
                    other_client.id,
                    format!("{} says \"{message}\"", character.name),
                );
            }
        }
    }

    Ok(())
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
        let message = handle_say("say Hello!");
        assert_eq!(message, Ok(Command::Say("Hello!".into())));

        let no_message = handle_say("say");
        assert_eq!(
            no_message,
            Err(ParseError::InvalidArguments("Say what?".into()))
        );

        let alias = handle_say("'Hello!");
        assert_eq!(alias, Ok(Command::Say("Hello!".into())));
    }

    #[test]
    fn sends_to_sender() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, say);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .name("Ramos")
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "say Hello!");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Ramos says \"Hello!\"");
    }

    #[test]
    fn sends_to_tile() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, say);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, sender_client_id, _) = PlayerBuilder::new()
            .tile(tile)
            .name("Flora")
            .build(&mut app);

        let (_, recipient_client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, sender_client_id, "say Hello!");
        app.update();

        let content = get_message_content(&mut app, recipient_client_id).unwrap();

        assert_eq!(content, "Flora says \"Hello!\"");
    }

    #[test]
    fn empty_message() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, say);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "say   ");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Say what?");
    }
}
