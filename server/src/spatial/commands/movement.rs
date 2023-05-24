use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    player::components::{Character, Client},
    spatial::{
        components::{Impassable, Position, Tile},
        utils::{offset_for_direction, view_for_tile},
    },
    visual::components::Sprite,
    world::resources::TileMap,
};

// USAGE: <direction>
pub fn movement(
    tile_map: Res<TileMap>,
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Position), With<Character>>,
    tiles: Query<(&Position, &Tile, &Sprite, Option<&Impassable>), Without<Character>>,
) {
    let regex = Regex::new(
        r"^(north|n|northeast|ne|east|e|southeast|se|south|s|southwest|sw|west|w|northwest|nw|up|u|down|d)$",
    )
    .unwrap();

    for (message, direction) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) if regex.is_match(text) => Some((message, text)),
        _ => None,
    }) {
        let Some((client, mut player_position)) = players.iter_mut().find(|(c, _)| c.0 == message.from) else {
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

            outbox.send_text(client.0, view_for_tile(tile, sprite))
        } else {
            outbox.send_text(client.0, "Something blocks your path.");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        spatial::components::Zone,
        test::{player_builder::PlayerBuilder, tile_builder::TileBuilder},
    };

    use super::*;

    #[test]
    fn test_movement() {
        let mut app = App::new();

        app.insert_resource(TileMap::default());
        app.add_event::<Inbox>();
        app.add_event::<Outbox>();
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
        assert!(matches!(response.content, Message::Text(_)));

        let response = match &response.content {
            Message::Text(text) => text,
            _ => panic!("Expected text message"),
        };

        assert!(response.contains("Southern Void"));
    }
}
