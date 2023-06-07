use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
    spatial::{
        components::{Position, Tile, Zone},
        utils::{offset_for_direction, view_for_tile},
    },
    visual::components::Sprite,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_movement(
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
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(Entity, &Client, &Character, &Parent)>,
    tiles: Query<(Entity, &Position, &Tile, &Sprite, &Parent)>,
    zones: Query<&Children, With<Zone>>,
) {
    for command in commands.iter() {
        if let Command::Movement(direction) = &command.command {
            let Some((player, client, character, tile)) = players.iter_mut().find(|(_, c, _, _)| c.id == command.from) else {
                debug!("Could not find player for client: {:?}", command.from);

                continue;
            };

            let Ok((_, position, _, _, zone)) = tiles.get(tile.get()) else {
                debug!("Could not get tile: {:?}", tile.get());

                continue;
            };

            let Ok(zone_tiles) = zones.get(zone.get()) else {
                debug!("Could not get zone: {:?}", zone.get());

                continue;
            };

            let Some(offset) = offset_for_direction(direction) else {
                continue;
            };

            let Some((target, tile, sprite)) = zone_tiles.iter().find_map(|child| {
                tiles.get(*child)
                    .ok()
                    .filter(|(_, p, _, _, _)| p.0 == position.0 + offset)
                    .map(|(e, _, t, s, _)| (e, t, s))
            }) else {
                outbox.send_text(client.id, "You can't go that way.");

                continue;
            };

            bevy.entity(player).set_parent(target);

            outbox.send_text(
                client.id,
                view_for_tile(tile, sprite, character.config.brief),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
        utils::{get_message_content, send_message},
    };

    use super::*;

    #[test]
    fn move_around() {
        let mut app = AppBuilder::new().build();

        app.add_system(movement);

        let zone = ZoneBuilder::new().build(&mut app);

        let start = TileBuilder::new()
            .position(IVec3::ZERO)
            .build(&mut app, zone);

        let destination = TileBuilder::new()
            .position(IVec3::new(0, 1, 0))
            .build(&mut app, zone);

        let (client_id, player) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "south");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), destination);
    }

    #[test]
    fn no_exit() {
        let mut app = AppBuilder::new().build();

        app.add_system(movement);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new()
            .position(IVec3::ZERO)
            .build(&mut app, zone);

        let (client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "south");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You can't go that way.");
    }
}
