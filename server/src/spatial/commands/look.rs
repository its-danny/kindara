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
    use crate::{
        spatial::components::Zone,
        test::{app_builder::AppBuilder, player_builder::PlayerBuilder, tile_builder::TileBuilder},
    };

    use super::*;

    #[test]
    fn test_look() {
        let mut app = AppBuilder::new();
        app.add_system(look);

        let tile = TileBuilder::new().name("Void").build(&mut app);

        app.world
            .resource_mut::<TileMap>()
            .insert((Zone::Void, IVec3::ZERO), tile);

        let (client_id, _) = PlayerBuilder::new().build(&mut app);

        app.world.resource_mut::<Events<Inbox>>().send(Inbox {
            from: client_id,
            content: Message::Text("look".into()),
        });

        app.update();

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

        assert_eq!(response, "x Void\nA vast, empty void.");
    }
}
