use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    combat::{
        components::{CombatState, QueuedAttack, Stats},
        events::{CombatEvent, CombatEventKind, CombatEventTrigger},
    },
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
        true => Ok(Command::Movement((content.into(), false))),
    }
}

#[derive(WorldQuery)]
pub struct NpcQuery {
    stats: &'static Stats,
    with_npc: With<Npc>,
}

#[derive(WorldQuery)]
pub struct TileQuery {
    entity: Entity,
    position: &'static Position,
    parent: &'static Parent,
    children: Option<&'static Children>,
    with_tile: With<Tile>,
}

#[sysfail(log)]
pub fn movement(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut proxy: EventWriter<ProxyCommand>,
    mut outbox: EventWriter<Outbox>,
    mut combat_events: EventWriter<CombatEvent>,
    mut players: Query<
        (
            Entity,
            &Client,
            Option<&CombatState>,
            Option<&QueuedAttack>,
            &Parent,
            Option<&Seated>,
        ),
        With<Online>,
    >,
    tiles: Query<TileQuery>,
    zones: Query<&Children, With<Zone>>,
    doors: Query<&Door>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Movement((direction, fleeing)) = &command.command {
            let (player, client, combat_state, queued_attack, tile, seated) = players
                .iter_mut()
                .find(|(_, c, _, _, _, _)| c.id == command.from)
                .context("Player not found")?;

            if combat_state.is_some() && !fleeing {
                combat_events.send(CombatEvent {
                    source: player,
                    trigger: CombatEventTrigger::Movement,
                    kind: CombatEventKind::AttemptFlee(direction.clone()),
                });

                continue;
            }

            if queued_attack.is_some() {
                bevy.entity(player).remove::<QueuedAttack>();
            }

            let player_tile = tiles.get(tile.get())?;
            let zone_tiles = zones.get(player_tile.parent.get())?;

            let Some(offset) = offset_for_direction(direction) else {
                continue;
            };

            let target = match get_target(&tiles, zone_tiles, &offset, player_tile.position) {
                Ok(target) => target,
                Err(err) => {
                    outbox.send_text(client.id, err.to_string());

                    continue;
                }
            };

            if let Err(err) = check_for_doors(&player_tile.children, &doors, &offset) {
                outbox.send_text(client.id, err.to_string());

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

#[derive(Error, Debug, PartialEq)]
enum TargetError {
    #[error("You can't go that way.")]
    NotFound,
}

fn get_target(
    tiles: &Query<TileQuery>,
    zone_tiles: &Children,
    offset: &IVec3,
    position: &Position,
) -> Result<Entity, TargetError> {
    let Some(target) = zone_tiles.iter().find_map(|child| {
        tiles
            .get(*child)
            .ok()
            .filter(|tile| tile.position.0 == position.0 + *offset)
            .map(|tile| tile.entity)
    }) else {
        Err(TargetError::NotFound)?
    };

    Ok(target)
}

#[derive(Error, Debug, PartialEq)]
enum DoorError {
    #[error("Your way is blocked.")]
    Blocked,
}

fn check_for_doors(
    siblings: &Option<&Children>,
    doors: &Query<&Door>,
    offset: &IVec3,
) -> Result<(), DoorError> {
    if let Some(siblings) = siblings {
        for child in siblings.iter() {
            if let Ok(door) = doors.get(*child) {
                if door.blocks == *offset && !door.is_open {
                    Err(DoorError::Blocked)?
                }
            }
        }
    }

    Ok(())
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
