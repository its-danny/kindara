use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    player::components::{Character, Client},
    spatial::{
        components::{Position, Tile},
        utils::view_for_tile,
    },
    visual::components::Sprite,
    world::resources::TileMap,
};

// USAGE: (look|l)
pub fn look(
    tile_map: Res<TileMap>,
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position), With<Character>>,
    tiles: Query<(&Tile, &Sprite)>,
) {
    let regex = Regex::new(r"^(look|l)$").unwrap();

    for (message, _) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) if regex.is_match(text) => Some((message, text)),
        _ => None,
    }) {
        let Some((client, player_position)) = players.iter().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        let Some((tile, sprite)) = tile_map
                .get(player_position.zone, player_position.coords)
                .and_then(|e| tiles.get(*e).ok()) else {
                    return;
                };

        outbox.send_text(client.0, view_for_tile(tile, sprite, false));
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
    fn test_look() {
        let mut app = App::new();

        app.insert_resource(TileMap::default());
        app.add_event::<Inbox>();
        app.add_event::<Outbox>();
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

        assert!(response.contains("A vast, empty void."));
    }
}
