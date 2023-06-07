use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
    spatial::{components::Tile, utils::view_for_tile},
    visual::components::Sprite,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_look(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(look|l)$").unwrap());

    if regex.is_match(content) {
        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Look,
        });

        true
    } else {
        false
    }
}

pub fn look(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Parent), With<Character>>,
    tiles: Query<(&Tile, &Sprite)>,
) {
    for command in commands.iter() {
        if let Command::Look = &command.command {
            let Some((client, tile)) = players.iter().find(|(c, _)| c.id == command.from) else {
                debug!("Could not find player for client: {:?}", command.from);

                continue;
            };

            let Ok((tile, sprite)) = tiles.get(tile.get()) else {
                debug!("Could not get parent: {:?}", tile.get());

                continue;
            };

            outbox.send_text(client.id, view_for_tile(tile, sprite, false));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        tile_builder::TileBuilder,
        utils::{get_message_content, send_message},
    };

    use super::*;

    #[test]
    fn sends_tile_info() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let tile = TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .build(&mut app);

        let (client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "x The Void\nA vast, empty void.");
    }
}
