use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Client, Online},
    spatial::components::{Door, Tile},
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_open(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^open( (?P<target>.+))?").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Open(target))
        }
    }
}

#[derive(WorldQuery)]
pub struct DoorQuery {
    entity: Entity,
    depiction: &'static Depiction,
    with_door: With<Door>,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct DoorMutQuery {
    entity: Entity,
    door: &'static mut Door,
}

#[sysfail(log)]
pub fn open(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Parent), With<Online>>,
    tiles: Query<Option<&Children>, With<Tile>>,
    doors: Query<DoorQuery>,
    mut doors_mut: Query<DoorMutQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Open(target) = &command.command {
            let (client, tile) = players
                .iter()
                .find(|(c, _)| c.id == command.from)
                .context("Player not found")?;

            let siblings = tiles.get(tile.get())?;

            let mut door = match get_door(target, &siblings, &doors, &mut doors_mut) {
                Ok(door) => door,
                Err(err) => {
                    outbox.send_text(client.id, err.to_string());

                    continue;
                }
            };

            door.is_open = true;

            outbox.send_text(client.id, "You open the door.");
        }
    }

    Ok(())
}

#[derive(Error, Debug, PartialEq)]
enum CloseError {
    #[error("Close what?")]
    NoTargetProvided,
    #[error("You don't see a {0} here.")]
    NotFound(String),
    #[error("You can't close that.")]
    NotClosable,
}

fn get_door<'a>(
    target: &Option<String>,
    siblings: &Option<&Children>,
    doors: &Query<DoorQuery>,
    doors_mut: &'a mut Query<DoorMutQuery>,
) -> Result<Mut<'a, Door>, CloseError> {
    let target = target.as_ref().ok_or(CloseError::NoTargetProvided)?;

    let door = siblings
        .iter()
        .flat_map(|siblings| siblings.iter())
        .filter_map(|sibling| doors.get(*sibling).ok())
        .find(|door| door.depiction.matches_query(&door.entity, target))
        .map(|door| door.entity)
        .ok_or_else(|| CloseError::NotFound(target.to_string()))?;

    let door = doors_mut
        .get_mut(door)
        .map_err(|_| CloseError::NotClosable)?;

    Ok(door.door)
}

#[cfg(test)]
mod tests {
    use crate::test::{
        app_builder::AppBuilder,
        item_builder::ItemBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
        utils::{get_message_content, send_message},
    };

    use super::*;

    #[test]
    fn parses() {
        let target = handle_open("open door");
        assert_eq!(target, Ok(Command::Open(Some("door".into()))));

        let no_target = handle_open("open");
        assert_eq!(no_target, Ok(Command::Open(None)));
    }

    #[test]
    fn opens_door() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, open);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let door = ItemBuilder::new().name("door").tile(tile).build(&mut app);
        app.world.entity_mut(door).insert(Door {
            is_open: false,
            blocks: IVec3::new(0, -1, 0),
        });

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "open door");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You open the door.");

        assert!(app.world.get::<Door>(door).unwrap().is_open);
    }
}
