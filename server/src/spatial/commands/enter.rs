use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    player::components::Client,
    spatial::{
        components::{Position, Tile, Transition},
        utils::view_for_tile,
    },
    visual::components::Sprite,
};

// USAGE: (enter) [target]
pub fn enter(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Position)>,
    transitions: Query<(&Position, &Transition), Without<Client>>,
    tiles: Query<(&Position, &Tile, &Sprite), Without<Client>>,
) {
    let regex = Regex::new(r"^(enter)( .+)?$").unwrap();

    for (message, captures) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) => regex.captures(text).map(|caps| (message, caps)),
        _ => None,
    }) {
        let Some((client, mut player_position)) = players.iter_mut().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        let target = captures.get(2).map(|m| m.as_str());

        let transition = transitions
            .iter()
            .filter(|(p, _)| p.zone == player_position.zone)
            .find(|(p, t)| {
                p.coords == player_position.coords
                    && target
                        .as_ref()
                        .map_or(true, |tag| t.tags.contains(&tag.trim().to_string()))
            });

        if let Some((_, transition)) = transition {
            player_position.zone = transition.zone;
            player_position.coords = transition.coords;

            if let Some((_, tile, sprite)) = tiles
                .iter()
                .filter(|(p, _, _)| p.zone == player_position.zone)
                .find(|(p, _, _)| p.coords == player_position.coords)
            {
                outbox.send_text(client.0, view_for_tile(tile, sprite))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        spatial::components::Zone,
        test::{
            player_builder::PlayerBuilder, tile_builder::TileBuilder,
            transition_builder::TransitionBuilder,
        },
        world::resources::TileMap,
    };

    use super::*;

    #[test]
    fn test_enter() {
        let mut app = App::new();

        app.insert_resource(TileMap::default());
        app.add_event::<Inbox>();
        app.add_event::<Outbox>();
        app.add_system(enter);

        let void_tile = TileBuilder::new()
            .name("Void")
            .coords(IVec3::ZERO)
            .build(&mut app);

        let movement_tile = TileBuilder::new()
            .name("Movement")
            .zone(Zone::Movement)
            .build(&mut app);

        TransitionBuilder::new()
            .target_zone(Zone::Movement)
            .target_coords(IVec3::ZERO)
            .build(&mut app);

        app.world
            .resource_mut::<TileMap>()
            .insert((Zone::Void, IVec3::ZERO), void_tile);

        app.world
            .resource_mut::<TileMap>()
            .insert((Zone::Movement, IVec3::ZERO), movement_tile);

        let (client_id, player) = PlayerBuilder::new().build(&mut app);

        app.world.resource_mut::<Events<Inbox>>().send(Inbox {
            from: client_id,
            content: Message::Text("enter".into()),
        });

        app.update();

        let updated_position = app.world.get::<Position>(player).unwrap();

        assert_eq!(updated_position.zone, Zone::Movement);
        assert_eq!(updated_position.coords, IVec3::ZERO);

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

        assert!(response.contains("Movement"));
    }
}
