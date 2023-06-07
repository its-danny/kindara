use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::{
        components::{Character, Client},
        permissions,
    },
    spatial::{
        components::{Position, Tile, Zone},
        utils::view_for_tile,
    },
    visual::components::Sprite,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_teleport(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(|| {
        Regex::new(r"^(teleport|tp) (?P<zone>here|(.+)) \(((?P<x>\d) (?P<y>\d) (?P<z>\d))\)$")
            .unwrap()
    });

    if let Some(captures) = regex.captures(content) {
        let region = captures.name("zone").map(|m| m.as_str()).unwrap_or("here");
        let x = captures
            .name("x")
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .unwrap_or_default();
        let y = captures
            .name("y")
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .unwrap_or_default();
        let z = captures
            .name("z")
            .and_then(|m| m.as_str().parse::<i32>().ok())
            .unwrap_or_default();

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Teleport((region.to_string(), (x, y, z))),
        });

        true
    } else {
        false
    }
}

pub fn teleport(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(Entity, &Client, &Character, &Parent)>,
    tiles: Query<(Entity, &Tile, &Sprite, &Position)>,
) {
    for command in commands.iter() {
        if let Command::Teleport((zone, (x, y, z))) = &command.command {
            let Some((player, client, character, tile)) = players.iter_mut().find(|(_, c, _, _)| c.id == command.from) else {
                debug!("Could not find player for client: {:?}", command.from);

                continue;
            };

            let Ok((_, _, _, position)) = tiles.get(tile.get()) else {
                debug!("Could not get parent: {:?}", tile.get());

                continue;
            };

            if !character.can(permissions::TELEPORT) {
                continue;
            }

            let coords = IVec3::new(*x, *y, *z);

            let zone = match zone.as_str() {
                "here" => Some(position.zone),
                "movement" => Some(Zone::Movement),
                "void" => Some(Zone::Void),
                _ => None,
            };

            let Some(zone) = zone else {
                outbox.send_text(client.id, "Invalid zone.");

                continue;
            };

            let Some((target, tile, sprite, _)) = tiles
                .iter()
                .find(|(_, _, _, p)| p.zone == zone && p.coords == coords)
            else {
                outbox.send_text(client.id, "Invalid location.");

                continue;
            };

            info!("Teleporting {} to {} in {}", character.name, coords, zone);

            bevy.entity(player).set_parent(target);

            outbox.send_text(client.id, view_for_tile(tile, sprite, false));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        player::permissions::TELEPORT,
        spatial::components::Zone,
        test::{
            app_builder::AppBuilder,
            player_builder::PlayerBuilder,
            tile_builder::TileBuilder,
            utils::{get_message_content, send_message},
        },
    };

    use super::*;

    #[test]
    fn teleports_zones() {
        let mut app = AppBuilder::new().build();
        app.add_system(teleport);

        let start = TileBuilder::new()
            .zone(Zone::Void)
            .coords(IVec3::ZERO)
            .build(&mut app);

        let destination = TileBuilder::new()
            .zone(Zone::Movement)
            .coords(IVec3::ZERO)
            .build(&mut app);

        let (client_id, player) = PlayerBuilder::new()
            .role(TELEPORT)
            .tile(start)
            .build(&mut app);

        send_message(&mut app, client_id, "teleport movement (0 0 0)");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), destination);
    }

    #[test]
    fn teleports_in_zone() {
        let mut app = AppBuilder::new().build();
        app.add_system(teleport);

        let start = TileBuilder::new().coords(IVec3::ZERO).build(&mut app);

        let destination = TileBuilder::new()
            .coords(IVec3::new(0, 1, 0))
            .build(&mut app);

        let (client_id, player) = PlayerBuilder::new()
            .role(TELEPORT)
            .tile(start)
            .build(&mut app);

        send_message(&mut app, client_id, "teleport here (0 1 0)");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), destination);
    }

    #[test]
    fn invalid_zone() {
        let mut app = AppBuilder::new().build();
        app.add_system(teleport);

        let tile = TileBuilder::new().build(&mut app);
        let (client_id, _) = PlayerBuilder::new()
            .role(TELEPORT)
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "teleport invalid (0 0 0)");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Invalid zone.");
    }

    #[test]
    fn invalid_location() {
        let mut app = AppBuilder::new().build();
        app.add_system(teleport);

        TileBuilder::new().build(&mut app);

        let tile = TileBuilder::new().build(&mut app);
        let (client_id, _) = PlayerBuilder::new()
            .role(TELEPORT)
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "teleport here (0 1 0)");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Invalid location.");
    }
}
