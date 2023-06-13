use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand, ProxyCommand},
    player::{
        components::{Character, Client, Online},
        permissions,
    },
    spatial::components::{Position, Tile, Zone},
    value_or_continue,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_teleport(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| {
        Regex::new(
            r#"^(teleport|tp)( (?P<zone>here|(.*?)))?( (\((?P<x>\d) (?P<y>\d) (?P<z>\d)\)))?$"#,
        )
        .unwrap()
    });

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let zone =
                captures
                    .name("zone")
                    .map(|m| m.as_str())
                    .ok_or(ParseError::InvalidArguments(
                        "Invalid zone name. Use `here` to teleport within the current zone.".into(),
                    ))?;

            let x = captures
                .name("x")
                .and_then(|m| m.as_str().parse::<i32>().ok())
                .ok_or(ParseError::InvalidArguments("Invalid coordinates.".into()))?;

            let y = captures
                .name("y")
                .and_then(|m| m.as_str().parse::<i32>().ok())
                .ok_or(ParseError::InvalidArguments("Invalid coordinates.".into()))?;

            let z = captures
                .name("z")
                .and_then(|m| m.as_str().parse::<i32>().ok())
                .ok_or(ParseError::InvalidArguments("Invalid coordinates.".into()))?;

            Ok(Command::Teleport((zone.to_lowercase(), (x, y, z))))
        }
    }
}

pub fn teleport(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut proxy: EventWriter<ProxyCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(Entity, &Client, &Character, &Parent), With<Online>>,
    tiles: Query<(Entity, &Position, &Parent), With<Tile>>,
    zones: Query<(&Zone, &Children)>,
) {
    for command in commands.iter() {
        if let Command::Teleport((zone, (x, y, z))) = &command.command {
            let (player, client, character, tile) =
                value_or_continue!(players.iter_mut().find(|(_, c, _, _)| c.id == command.from));
            let (_, _, here) = value_or_continue!(tiles.get(tile.get()).ok());
            let (here, _) = value_or_continue!(zones.get(here.get()).ok());

            if !character.can(permissions::TELEPORT) {
                continue;
            }

            let position = IVec3::new(*x, *y, *z);

            let Some((zone, zone_tiles)) = zones.iter().find(|(z, _)| {
                match zone {
                    name if name == "here" => z.name == here.name,
                    name => z.name.to_lowercase() == *name,
                }
            }) else  {
                outbox.send_text(client.id, format!("Zone \"{}\" not found.", zone));

                continue;
            };

            let Some(target) = zone_tiles.iter().find_map(|child| {
                tiles.get(*child)
                    .ok()
                    .filter(|( _, p, _)| p.0 == position)
                    .map(|(e, _, _)| e)
            }) else {
                outbox.send_text(client.id, format!("No tile found at {}", position));

                continue;
            };

            info!(
                "Teleporting {} to {} in {}",
                character.name, position, zone.name
            );

            bevy.entity(player).set_parent(target);

            proxy.send(ProxyCommand(ParsedCommand {
                from: client.id,
                command: Command::Look(None),
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        player::permissions::TELEPORT,
        test::{
            app_builder::AppBuilder,
            player_builder::PlayerBuilder,
            tile_builder::{TileBuilder, ZoneBuilder},
            utils::{get_message_content, send_message},
        },
    };

    use super::*;

    #[test]
    fn teleports_zones() {
        let mut app = AppBuilder::new().build();
        app.add_system(teleport);

        let start_zone = ZoneBuilder::new().build(&mut app);
        let start = TileBuilder::new()
            .position(IVec3::ZERO)
            .build(&mut app, start_zone);

        let destination_zone = ZoneBuilder::new().name("Uruk").build(&mut app);
        let destination = TileBuilder::new()
            .position(IVec3::ZERO)
            .build(&mut app, destination_zone);

        let (player, client_id, _) = PlayerBuilder::new()
            .role(TELEPORT)
            .tile(start)
            .build(&mut app);

        send_message(&mut app, client_id, "teleport uruk (0 0 0)");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), destination);
    }

    #[test]
    fn teleports_in_zone() {
        let mut app = AppBuilder::new().build();
        app.add_system(teleport);

        let zone = ZoneBuilder::new().build(&mut app);

        let start = TileBuilder::new()
            .position(IVec3::ZERO)
            .build(&mut app, zone);

        let destination = TileBuilder::new()
            .position(IVec3::new(0, 1, 0))
            .build(&mut app, zone);

        let (player, client_id, _) = PlayerBuilder::new()
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

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .role(TELEPORT)
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "teleport invalid (0 0 0)");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Zone \"invalid\" not found.");
    }

    #[test]
    fn invalid_location() {
        let mut app = AppBuilder::new().build();
        app.add_system(teleport);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .role(TELEPORT)
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "teleport here (0 1 0)");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "No tile found at [0, 1, 0]");
    }
}
