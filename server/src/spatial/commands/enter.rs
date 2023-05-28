use bevy::prelude::*;
use bevy_nest::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
    spatial::{
        components::{Position, Tile, Transition},
        utils::view_for_tile,
    },
    visual::components::Sprite,
};
static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(enter)(?P<transition> .+)?$").unwrap());

pub fn parse_enter(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if let Some(captures) = REGEX.captures(content) {
        let target = captures.name("transition").map(|m| m.as_str().to_string());

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Enter(target),
        });

        true
    } else {
        false
    }
}

pub fn enter(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Position), With<Character>>,
    transitions: Query<(&Position, &Transition), Without<Client>>,
    tiles: Query<(&Position, &Tile, &Sprite), Without<Client>>,
) {
    for command in commands.iter() {
        if let Command::Enter(target) = &command.command {
            let Some((client, mut player_position)) = players.iter_mut().find(|(c, _)| c.id == command.from) else {
                return;
            };

            let transition = transitions.iter().find(|(p, t)| {
                p.zone == player_position.zone
                    && p.coords == player_position.coords
                    && target
                        .as_ref()
                        .map_or(true, |tag| t.tags.contains(&tag.trim().to_string()))
            });

            if let Some((_, transition)) = transition {
                player_position.zone = transition.zone;
                player_position.coords = transition.coords;

                if let Some((_, tile, sprite)) = tiles.iter().find(|(p, _, _)| {
                    p.zone == player_position.zone && p.coords == player_position.coords
                }) {
                    outbox.send_text(client.id, view_for_tile(tile, sprite, false))
                }
            } else {
                outbox.send_text(client.id, "Enter what?");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        spatial::components::Zone,
        test::{
            app_builder::AppBuilder, player_builder::PlayerBuilder, tile_builder::TileBuilder,
            transition_builder::TransitionBuilder,
        },
        world::resources::TileMap,
    };

    use super::*;

    #[test]
    fn test_enter() {
        let mut app = AppBuilder::new();
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
            .tags(&vec!["movement"])
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
            content: Message::Text("enter movement".into()),
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

        let response = match &response.content {
            Message::Text(text) => text,
            _ => panic!("Expected text message"),
        };

        assert!(response.contains("Movement"));
    }
}
