use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
    spatial::components::Tile,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_say(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(|| Regex::new(r"^((say |')(?P<message>.+))$").unwrap());

    if let Some(captures) = regex.captures(content) {
        let message = captures
            .name("message")
            .map(|m| m.as_str().trim())
            .unwrap_or("");

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Say(message.into()),
        });

        true
    } else {
        false
    }
}

pub fn say(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent)>,
    tiles: Query<&Children, With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Say(message) = &command.command {
            let Some((client, character, tile)) = players.iter().find(|(c, _, _)| c.id == command.from) else {
                debug!("Could not find player for client: {:?}", command.from);

                continue;
            };

            let Ok(siblings) = tiles.get(tile.get()) else {
                debug!("Could not get parent: {:?}", tile.get());

                continue;
            };

            if message.is_empty() {
                outbox.send_text(client.id, "Say what?");

                continue;
            }

            outbox.send_text(client.id, format!("You say \"{message}\""));

            for (other_client, _, _) in siblings.iter().filter_map(|c| players.get(*c).ok()) {
                if other_client.id != client.id {
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

        let tile = TileBuilder::new().build(&mut app);

        let (client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "say Hello!");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You say \"Hello!\"");
    }

    #[test]
    fn sends_to_tile() {
        let mut app = AppBuilder::new().build();
        app.add_system(say);

        let tile = TileBuilder::new().build(&mut app);

        let (sender_client_id, _) = PlayerBuilder::new()
            .tile(tile)
            .name("Flora")
            .build(&mut app);
        let (recipient_client_id, _) = PlayerBuilder::new()
            .tile(tile)
            .name("Salus")
            .build(&mut app);

        send_message(&mut app, sender_client_id, "say Hello!");
        app.update();

        let content = get_message_content(&mut app, recipient_client_id);

        assert_eq!(content, "Flora says \"Hello!\"");
    }

    #[test]
    fn empty_message() {
        let mut app = AppBuilder::new().build();
        app.add_system(say);

        let tile = TileBuilder::new().build(&mut app);

        let (client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "say   ");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Say what?");
    }
}
