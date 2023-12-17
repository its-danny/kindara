use std::sync::OnceLock;

use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Character, Client, Online},
    spatial::components::{Tile, Zone},
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_yell(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r#"^(yell( |$)|" ?)(?P<message>.*)?$"#).unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let message = captures
                .name("message")
                .map(|m| m.as_str().trim())
                .filter(|m| !m.is_empty())
                .ok_or(ParseError::InvalidArguments("Yell what?".into()))?;

            Ok(Command::Yell(message.into()))
        }
    }
}

#[sysfail(log)]
pub fn yell(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent), With<Online>>,
    tiles: Query<&Parent, With<Tile>>,
    zones: Query<&Children, With<Zone>>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Yell(message) = &command.command {
            let (_, character, tile) = players
                .iter()
                .find(|(c, _, _)| c.id == command.from)
                .context("Player not found")?;

            let zone = tiles.get(tile.get())?;
            let zone_tiles = zones.get(zone.get())?;

            for (client, _, _) in players.iter().filter(|(_, _, t)| zone_tiles.contains(t)) {
                outbox.send_text(client.id, format!("{} yells \"{message}\"", character.name));
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
        let message = handle_yell("yell Hey!");
        assert_eq!(message, Ok(Command::Yell("Hey!".into())));

        let no_message = handle_yell("yell");
        assert_eq!(
            no_message,
            Err(ParseError::InvalidArguments("Yell what?".into()))
        );

        let alias = handle_yell("\" Hey!");
        assert_eq!(alias, Ok(Command::Yell("Hey!".into())));
    }

    #[test]
    fn sends_to_sender() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, yell);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .name("Ramos")
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "yell Hello!");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Ramos yells \"Hello!\"");
    }

    #[test]
    fn sends_to_zone() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, yell);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile_one = TileBuilder::new().build(&mut app, zone);
        let tile_two = TileBuilder::new().build(&mut app, zone);

        let (_, sender_client_id, _) = PlayerBuilder::new()
            .tile(tile_one)
            .name("Flora")
            .build(&mut app);

        let (_, recipient_client_id, _) = PlayerBuilder::new().tile(tile_two).build(&mut app);

        send_message(&mut app, sender_client_id, "yell Hello!");
        app.update();

        let content = get_message_content(&mut app, recipient_client_id).unwrap();

        assert_eq!(content, "Flora yells \"Hello!\"");
    }

    #[test]
    fn empty_message() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, yell);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "yell   ");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Yell what?");
    }
}
