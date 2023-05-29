use bevy::prelude::*;
use bevy_nest::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
    spatial::{
        components::{Position, Tile},
        utils::view_for_tile,
    },
    visual::components::Sprite,
    world::resources::TileMap,
};

static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(look|l)$").unwrap());

pub fn parse_look(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if REGEX.is_match(content) {
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
    tile_map: Res<TileMap>,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position), With<Character>>,
    tiles: Query<(&Tile, &Sprite)>,
) {
    for command in commands.iter() {
        if let Command::Look = &command.command {
            let Some((client, player_position)) = players.iter().find(|(c, _)| c.id == command.from) else {
                return;
            };

            let Some((tile, sprite)) = tile_map
                .get(player_position.zone, player_position.coords)
                .and_then(|e| tiles.get(*e).ok()) else {
                    return;
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
        let mut app = AppBuilder::new();
        app.add_system(look);

        TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .build(&mut app);

        let (client_id, _) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "look");

        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "x The Void\nA vast, empty void.");
    }
}
