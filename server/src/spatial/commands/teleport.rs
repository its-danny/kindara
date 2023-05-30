use bevy::prelude::*;
use bevy_nest::prelude::*;
use once_cell::sync::Lazy;
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
    world::resources::TileMap,
};

static REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(teleport|tp) (?P<zone>here|(.+)) \(((?P<x>\d) (?P<y>\d) (?P<z>\d))\)$").unwrap()
});

pub fn parse_teleport(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if let Some(captures) = REGEX.captures(content) {
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
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Position, &Character)>,
    tile_map: Res<TileMap>,
    tiles: Query<(&Tile, &Sprite), Without<Character>>,
) {
    for command in commands.iter() {
        if let Command::Teleport((zone, (x, y, z))) = &command.command {
            let Some((client, mut player_position, character)) = players.iter_mut().find(|(c, _, _)| c.id == command.from) else {
                return;
            };

            if !character.can(permissions::TELEPORT) {
                return;
            }

            let coords = IVec3::new(*x, *y, *z);

            let zone = match zone.as_str() {
                "here" => Some(player_position.zone),
                "movement" => Some(Zone::Movement),
                "void" => Some(Zone::Void),
                _ => None,
            };

            if let Some(zone) = zone {
                let tile_sprite_option =
                    tile_map.get(zone, coords).and_then(|e| tiles.get(*e).ok());

                if let Some((tile, sprite)) = tile_sprite_option {
                    info!("Teleporting {} to {} in {}", character.name, coords, zone);

                    player_position.zone = zone;
                    player_position.coords = coords;

                    outbox.send_text(
                        client.id,
                        view_for_tile(tile, sprite, character.config.brief),
                    );
                } else {
                    outbox.send_text(client.id, "Invalid location.");
                }
            } else {
                outbox.send_text(client.id, "Invalid zone.");
            }
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

        TileBuilder::new()
            .zone(Zone::Void)
            .coords(IVec3::ZERO)
            .build(&mut app);

        TileBuilder::new()
            .zone(Zone::Movement)
            .coords(IVec3::ZERO)
            .build(&mut app);

        let (client_id, player) = PlayerBuilder::new()
            .role(TELEPORT)
            .zone(Zone::Void)
            .build(&mut app);

        send_message(&mut app, client_id, "teleport movement (0 0 0)");

        app.update();

        let updated_position = app.world.get::<Position>(player).unwrap();

        assert_eq!(updated_position.zone, Zone::Movement);
        assert_eq!(updated_position.coords, IVec3::ZERO);
    }

    #[test]
    fn teleports_in_zone() {
        let mut app = AppBuilder::new().build();
        app.add_system(teleport);

        TileBuilder::new().coords(IVec3::ZERO).build(&mut app);

        TileBuilder::new()
            .coords(IVec3::new(0, 1, 0))
            .build(&mut app);

        let (client_id, player) = PlayerBuilder::new().role(TELEPORT).build(&mut app);

        send_message(&mut app, client_id, "teleport here (0 1 0)");

        app.update();

        let updated_position = app.world.get::<Position>(player).unwrap();

        assert_eq!(updated_position.zone, Zone::Void);
        assert_eq!(updated_position.coords, IVec3::new(0, 1, 0));
    }

    #[test]
    fn invalid_zone() {
        let mut app = AppBuilder::new().build();
        app.add_system(teleport);

        let (client_id, _) = PlayerBuilder::new().role(TELEPORT).build(&mut app);

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

        let (client_id, _) = PlayerBuilder::new().role(TELEPORT).build(&mut app);

        send_message(&mut app, client_id, "teleport here (0 1 0)");

        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Invalid location.");
    }
}
