use bevy::prelude::*;
use bevy_nest::prelude::*;
use once_cell::sync::Lazy;
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

static REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(north|n|northeast|ne|east|e|southeast|se|south|s|southwest|sw|west|w|northwest|nw|up|u|down|d)$").unwrap()
});

pub fn parse_movement(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if REGEX.is_match(content) {
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
    use crate::{
        spatial::components::Zone,
        test::{app_builder::AppBuilder, player_builder::PlayerBuilder, tile_builder::TileBuilder},
    };

    use super::*;

    #[test]
    fn test_movement() {
        let mut app = AppBuilder::new();

        app.add_system(movement);

        let tile_north = TileBuilder::new()
            .name("Northern Void")
            .coords(IVec3::ZERO)
            .build(&mut app);

        let tile_south = TileBuilder::new()
            .name("Southern Void")
            .coords(IVec3::new(0, 1, 0))
            .build(&mut app);

        app.world
            .resource_mut::<TileMap>()
            .insert((Zone::Void, IVec3::ZERO), tile_north);

        app.world
            .resource_mut::<TileMap>()
            .insert((Zone::Void, IVec3::new(0, 1, 0)), tile_south);

        let (client_id, player) = PlayerBuilder::new().build(&mut app);

        app.world.resource_mut::<Events<Inbox>>().send(Inbox {
            from: client_id,
            content: Message::Text("south".into()),
        });

        app.update();

        assert_eq!(
            app.world.get::<Position>(player).unwrap().coords,
            IVec3::new(0, 1, 0)
        );

        let outbox_events = app.world.resource::<Events<Outbox>>();
        let mut outbox_reader = outbox_events.get_reader();

        let response = outbox_reader
            .iter(outbox_events)
            .next()
            .expect("Expected response");

        assert_eq!(response.to, client_id);

        let response = match &response.content {
            Message::Text(text) => text,
            _ => panic!("Expected text message"),
        };

        assert!(response.contains("Southern Void"));
    }
}
