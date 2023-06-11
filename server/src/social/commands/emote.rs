use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client, Online},
    spatial::components::Tile,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_emote(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(|| Regex::new(r"^((emote |;)(?P<action>.+))$").unwrap());

    if let Some(captures) = regex.captures(content) {
        let action = captures
            .name("action")
            .map(|m| m.as_str().trim())
            .unwrap_or("");

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Emote(action.into()),
        });

        true
    } else {
        false
    }
}

pub fn emote(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Emote(action) = &command.command {
            let Some((client, character, tile)) = players.iter().find(|(c, _, _)| c.id == command.from) else {
                debug!("Could not find authenticated client: {:?}", command.from);

                continue;
            };

            let Ok(siblings) = tiles.get(tile.get()) else {
                debug!("Could not get tile: {:?}", tile.get());

                continue;
            };

            if action.is_empty() {
                outbox.send_text(client.id, "Do what?");

                continue;
            }

            for (other_client, _, _) in siblings.iter().filter_map(|c| players.get(*c).ok()) {
                outbox.send_text(other_client.id, format!("{} {action}", character.name));
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
    fn sends_to_sender() {
        let mut app = AppBuilder::new().build();
        app.add_system(emote);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .name("Ramos")
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "emote waves.");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Ramos waves.");
    }

    #[test]
    fn sends_to_tile() {
        let mut app = AppBuilder::new().build();
        app.add_system(emote);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, sender_client_id, _) = PlayerBuilder::new()
            .tile(tile)
            .name("Flora")
            .build(&mut app);

        let (_, recipient_client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, sender_client_id, "emote waves.");
        app.update();

        let content = get_message_content(&mut app, recipient_client_id);

        assert_eq!(content, "Flora waves.");
    }

    #[test]
    fn empty_message() {
        let mut app = AppBuilder::new().build();
        app.add_system(emote);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "emote   ");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Do what?");
    }
}
