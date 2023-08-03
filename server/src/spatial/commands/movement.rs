use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand, ProxyCommand},
    player::components::{Character, Client, Online},
    spatial::{
        components::{Action, Position, Seated, Tile, Zone},
        utils::offset_for_direction,
    },
    value_or_continue,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_movement(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(||
        Regex::new(r"^(north|n|northeast|ne|east|e|southeast|se|south|s|southwest|sw|west|w|northwest|nw|up|u|down|d)$").unwrap()
    );

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Movement(content.into())),
    }
}

pub fn movement(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut proxy: EventWriter<ProxyCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(Entity, &Client, &Character, &Parent, Option<&Seated>), With<Online>>,
    tiles: Query<(Entity, &Position, &Parent), With<Tile>>,
    zones: Query<&Children, With<Zone>>,
) {
    for command in commands.iter() {
        if let Command::Movement(direction) = &command.command {
            let (player, client, character, tile, seated) = value_or_continue!(players
                .iter_mut()
                .find(|(_, c, _, _, _)| c.id == command.from));

            if character.state.is_combat() {
                outbox.send_text(client.id, "You can't move while in combat.");

                continue;
            }

            let (_, position, zone) = value_or_continue!(tiles.get(tile.get()).ok());
            let zone_tiles = value_or_continue!(zones.get(zone.get()).ok());

            let Some(offset) = offset_for_direction(direction) else {
                continue;
            };

            let Some(target) = zone_tiles.iter().find_map(|child| {
                tiles.get(*child)
                    .ok()
                    .filter(|(_, p, _)| p.0 == position.0 + offset)
                    .map(|(e, _, _)| e)
            }) else {
                outbox.send_text(client.id, "You can't go that way.");

                continue;
            };

            bevy.entity(player).set_parent(target);

            if seated.is_some() {
                bevy.entity(player).remove::<Seated>();
                bevy.entity(player).remove::<Action>();
            }

            proxy.send(ProxyCommand(ParsedCommand {
                from: client.id,
                command: Command::Look(None),
            }));
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

        app.add_systems(Update, movement);

        let zone = ZoneBuilder::new().build(&mut app);

        let start = TileBuilder::new()
            .position(IVec3::ZERO)
            .build(&mut app, zone);

        let destination = TileBuilder::new()
            .position(IVec3::new(0, 1, 0))
            .build(&mut app, zone);

        let (player, client_id, _) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "south");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), destination);
    }

    #[test]
    fn no_exit() {
        let mut app = AppBuilder::new().build();

        app.add_systems(Update, movement);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new()
            .position(IVec3::ZERO)
            .build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "south");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You can't go that way.");
    }
}
