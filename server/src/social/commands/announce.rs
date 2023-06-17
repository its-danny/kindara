use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;
use vari::vformat;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    keycard::{Keycard, ANNOUNCE},
    player::components::{Client, Online},
    value_or_continue,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_announce(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^announce( (?P<message>.*))?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let message = captures
                .name("message")
                .map(|m| m.as_str().trim())
                .filter(|m| !m.is_empty())
                .ok_or(ParseError::InvalidArguments("Announce what?".into()))?;

            Ok(Command::Announce(message.into()))
        }
    }
}

pub fn announce(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Keycard), With<Online>>,
) {
    for command in commands.iter() {
        if let Command::Announce(message) = &command.command {
            let (_, keycard) =
                value_or_continue!(players.iter().find(|(c, _)| c.id == command.from));

            if !keycard.can(ANNOUNCE) {
                continue;
            }

            for (client, _) in players.iter() {
                outbox.send_text(
                    client.id,
                    vformat!("[[$yellow]announcement[$reset]]: {message}"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use vari::util::NoAnsi;

    use super::*;
    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
        utils::{get_message_content, send_message},
    };

    #[test]
    fn sends_to_everyone() {
        let mut app = AppBuilder::new().build();
        app.add_system(announce);

        let zone_one = ZoneBuilder::new().build(&mut app);
        let tile_one = TileBuilder::new().build(&mut app, zone_one);

        let zone_two = ZoneBuilder::new().build(&mut app);
        let tile_two = TileBuilder::new().build(&mut app, zone_two);

        let (_, sender_client_id, _) = PlayerBuilder::new()
            .tile(tile_one)
            .role(Keycard::admin())
            .build(&mut app);

        let (_, recipient_client_id, _) = PlayerBuilder::new().tile(tile_two).build(&mut app);

        send_message(&mut app, sender_client_id, "announce Hello!");
        app.update();

        let content = get_message_content(&mut app, recipient_client_id).unwrap();

        assert_eq!(content.no_ansi(), "[announcement]: Hello!");
    }

    #[test]
    fn empty_message() {
        let mut app = AppBuilder::new().build();
        app.add_system(announce);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .role(Keycard::admin())
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "announce   ");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Announce what?");
    }

    #[test]
    fn forbidden() {
        let mut app = AppBuilder::new().build();
        app.add_system(announce);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "announce Hello!");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert!(content.is_none());
    }
}
