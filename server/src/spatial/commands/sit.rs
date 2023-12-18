use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::Interactions,
    items::components::Seat,
    paint,
    player::components::{Client, Online},
    spatial::components::{Action, Seated, Tile},
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_sit(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^sit( (?P<target>.+))?").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Sit(target))
        }
    }
}

#[derive(WorldQuery)]
pub struct SeatQuery {
    entity: Entity,
    interactions: &'static Interactions,
    seat: &'static Seat,
    depiction: &'static Depiction,
}

#[sysfail(log)]
pub fn sit(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(Entity, &Client, &Parent), With<Online>>,
    tiles: Query<Option<&Children>, With<Tile>>,
    seats: Query<SeatQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Sit(target) = &command.command {
            let (player, client, tile) = players
                .iter_mut()
                .find(|(_, c, _)| c.id == command.from)
                .context("Player not found")?;

            if target.is_none() {
                sit_on_floor(&mut bevy, player);
                outbox.send_text(client.id, "You sit on the floor.");
                continue;
            }

            let siblings = tiles.get(tile.get()).ok().flatten();

            match find_seat(&siblings, target, &seats) {
                Ok(seat) => {
                    sit_on_seat(&mut bevy, player, seat);
                    outbox.send_text(client.id, paint!("You sit {}.", seat.phrase));
                }
                Err(err) => {
                    outbox.send_text(client.id, err.to_string());
                }
            }
        }
    }

    Ok(())
}

#[derive(Error, Debug, PartialEq)]
enum SitError {
    #[error("You can't sit there.")]
    NoSeat,
}

fn find_seat<'a>(
    siblings: &Option<&Children>,
    target: &Option<String>,
    seats: &'a Query<SeatQuery>,
) -> Result<&'a Seat, SitError> {
    target
        .as_ref()
        .and_then(|target| {
            siblings
                .iter()
                .flat_map(|siblings| siblings.iter())
                .filter_map(|sibling| seats.get(*sibling).ok())
                .find(|child| child.depiction.matches_query(&child.entity, target))
                .map(|child| child.seat)
        })
        .ok_or(SitError::NoSeat)
}

fn sit_on_floor(bevy: &mut Commands, player: Entity) {
    bevy.entity(player).insert(Action("on the floor".into()));
    bevy.entity(player).insert(Seated);
}

fn sit_on_seat(bevy: &mut Commands, player: Entity, seat: &Seat) {
    bevy.entity(player).insert(Action(seat.phrase.clone()));
    bevy.entity(player).insert(Seated);
}

#[cfg(test)]
mod tests {
    use crate::{
        interact::components::Interaction,
        test::{
            app_builder::AppBuilder,
            item_builder::ItemBuilder,
            player_builder::PlayerBuilder,
            tile_builder::{TileBuilder, ZoneBuilder},
            utils::{get_message_content, send_message},
        },
    };

    use super::*;

    #[test]
    fn parses() {
        let target = handle_sit("sit chair");
        assert_eq!(target, Ok(Command::Sit(Some("chair".into()))));

        let no_target = handle_sit("sit");
        assert_eq!(no_target, Ok(Command::Sit(None)));
    }

    #[test]
    fn sit_on_the_floor() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, sit);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "sit");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You sit on the floor.");

        assert!(app.world.entity(player).get::<Action>().is_some());
    }

    #[test]
    fn sit_on_chair() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, sit);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);
        let chair = ItemBuilder::new()
            .name("chair")
            .tile(tile)
            .interactions(vec![Interaction::Sit])
            .build(&mut app);

        app.world.entity_mut(chair).insert(Seat {
            phrase: "on the chair".into(),
        });

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "sit chair");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You sit on the chair.");

        assert_eq!(
            app.world.entity(player).get::<Action>().unwrap().0,
            "on the chair"
        );
    }
}
