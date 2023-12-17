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

pub fn handle_emote(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(emote( |$)|; ?)(?P<action>.*)?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let action = captures
                .name("action")
                .map(|a| a.as_str().trim())
                .filter(|a| !a.is_empty())
                .ok_or(ParseError::InvalidArguments("Do what?".into()))?;

            Ok(Command::Emote(action.into()))
        }
    }
}

#[sysfail(log)]
pub fn emote(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Emote(action) = &command.command {
            let (_, character, tile) = players
                .iter()
                .find(|(c, _, _)| c.id == command.from)
                .context("Player not found")?;

            let siblings = tiles.get(tile.get())?;

            for (other_client, _, _) in siblings.iter().filter_map(|c| players.get(*c).ok()) {
                outbox.send_text(other_client.id, format!("{} {action}", character.name));
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
        let message = handle_emote("emote waves");
        assert_eq!(message, Ok(Command::Emote("waves".into())));

        let no_message = handle_emote("emote");
        assert_eq!(
            no_message,
            Err(ParseError::InvalidArguments("Do what?".into()))
        );

        let alias = handle_emote(";waves");
        assert_eq!(alias, Ok(Command::Emote("waves".into())));
    }

    #[test]
    fn sends_to_sender() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, emote);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .name("Ramos")
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "emote waves.");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Ramos waves.");
    }

    #[test]
    fn sends_to_tile() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, emote);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, sender_client_id, _) = PlayerBuilder::new()
            .tile(tile)
            .name("Flora")
            .build(&mut app);

        let (_, recipient_client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, sender_client_id, "emote waves.");
        app.update();

        let content = get_message_content(&mut app, recipient_client_id).unwrap();

        assert_eq!(content, "Flora waves.");
    }

    #[test]
    fn empty_message() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, emote);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "emote   ");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Do what?");
    }
}
