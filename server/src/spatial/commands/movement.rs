use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
    spatial::{
        components::{Impassable, Position, Tile},
        utils::{offset_for_direction, view_for_tile},
    },
    visual::components::Sprite,
    world::resources::TileMap,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn parse_movement(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(||
        Regex::new(r"^(north|n|northeast|ne|east|e|southeast|se|south|s|southwest|sw|west|w|northwest|nw|up|u|down|d)$").unwrap()
    );

    if regex.is_match(content) {
        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Movement(content.to_string()),
        });

        true
    } else {
        false
    }
}

pub fn movement(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Position, &Character)>,
    tile_map: Res<TileMap>,
    tiles: Query<(&Position, &Tile, &Sprite, Option<&Impassable>), Without<Character>>,
) {
    for command in commands.iter() {
        if let Command::Movement(direction) = &command.command {
            let Some((client, mut player_position, character)) = players.iter_mut().find(|(c, _, _)| c.id == command.from) else {
                return;
            };

            let Some(offset) = offset_for_direction(direction) else {
                return;
            };

            let Some((tile_position, tile, sprite, impassable)) = tile_map
                .get(player_position.zone, player_position.coords + offset)
                .and_then(|e| tiles.get(*e).ok()) else {
                    outbox.send_text(client.id, "You can't go that way.");

                    return;
                };

            if impassable.is_none() {
                player_position.coords = tile_position.coords;

                outbox.send_text(
                    client.id,
                    view_for_tile(tile, sprite, character.config.brief),
                )
            } else {
                outbox.send_text(client.id, "Something blocks your path.");
            }
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
    fn move_around() {
        let mut app = AppBuilder::new().build();

        app.add_system(movement);

        TileBuilder::new().coords(IVec3::ZERO).build(&mut app);

        TileBuilder::new()
            .coords(IVec3::new(0, 1, 0))
            .build(&mut app);

        let (client_id, player) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "south");
        app.update();

        assert_eq!(
            app.world.get::<Position>(player).unwrap().coords,
            IVec3::new(0, 1, 0)
        );
    }

    #[test]
    fn no_exit() {
        let mut app = AppBuilder::new().build();

        app.add_system(movement);

        TileBuilder::new().coords(IVec3::ZERO).build(&mut app);

        let (client_id, _) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "south");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You can't go that way.");
    }

    #[test]
    fn impassable_tile() {
        let mut app = AppBuilder::new().build();

        app.add_system(movement);

        TileBuilder::new().coords(IVec3::ZERO).build(&mut app);

        TileBuilder::new()
            .coords(IVec3::new(0, 1, 0))
            .impassable(true)
            .build(&mut app);

        let (client_id, _) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "south");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Something blocks your path.");
    }
}
