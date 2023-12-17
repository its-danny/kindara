use std::sync::OnceLock;

use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    combat::components::{Attributes, InCombat, QueuedAttack},
    input::events::{Command, ParseError, ParsedCommand, ProxyCommand},
    npc::components::Npc,
    player::components::{Client, Online},
    spatial::{
        components::{Door, Position, Seated, Tile, Zone},
        utils::offset_for_direction,
    },
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

#[sysfail(log)]
pub fn movement(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut proxy: EventWriter<ProxyCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<
        (
            Entity,
            &Client,
            Option<&InCombat>,
            Option<&QueuedAttack>,
            &Parent,
            Option<&Seated>,
        ),
        With<Online>,
    >,
    npc_attrs: Query<&Attributes, With<Npc>>,
    tiles: Query<(Entity, &Position, &Parent, Option<&Children>), With<Tile>>,
    zones: Query<&Children, With<Zone>>,
    doors: Query<&Door>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Movement(direction) = &command.command {
            let (player, client, in_combat, queued_attack, tile, seated) = players
                .iter_mut()
                .find(|(_, c, _, _, _, _)| c.id == command.from)
                .context("Player not found")?;

            if let Some(in_combat) = in_combat {
                let attrs = npc_attrs.get(in_combat.target)?;

                if !in_combat.can_move(attrs, &queued_attack) {
                    outbox.send_text(client.id, "You failed to get away.");

                    continue;
                }
            }

            let (_, position, zone, siblings) = tiles.get(tile.get())?;
            let zone_tiles = zones.get(zone.get())?;

            let Some(offset) = offset_for_direction(direction) else {
                continue;
            };

            let Some(target) = zone_tiles.iter().find_map(|child| {
                tiles
                    .get(*child)
                    .ok()
                    .filter(|(_, p, _, _)| p.0 == position.0 + offset)
                    .map(|(e, _, _, _)| e)
            }) else {
                outbox.send_text(client.id, "You can't go that way.");

                continue;
            };

            if blocked_by_door(&siblings, &doors, &offset) {
                outbox.send_text(client.id, "Your way is blocked.");

                continue;
            }

            if seated.is_some() {
                proxy.send(ProxyCommand(ParsedCommand {
                    from: client.id,
                    command: Command::Stand,
                }));
            }

            bevy.entity(player).set_parent(target);

            proxy.send(ProxyCommand(ParsedCommand {
                from: client.id,
                command: Command::Look(None),
            }));
        }
    }

    Ok(())
}

/// Check if the target tile is blocked by a closed door.
fn blocked_by_door(siblings: &Option<&Children>, doors: &Query<&Door>, offset: &IVec3) -> bool {
    if let Some(siblings) = siblings {
        for child in siblings.iter() {
            if let Ok(door) = doors.get(*child) {
                return door.blocks == *offset && !door.is_open;
            }
        }
    }

    false
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
