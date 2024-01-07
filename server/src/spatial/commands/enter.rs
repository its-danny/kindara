use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    combat::components::CombatState,
    input::events::{Command, ParseError, ParsedCommand, ProxyCommand},
    player::components::{Client, Online},
    spatial::components::{Position, Tile, Transition, Zone},
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_enter(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^enter( (?P<transition>.+))?").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let transition = captures
                .name("transition")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Enter(transition))
        }
    }
}

#[derive(WorldQuery)]
pub struct TransitionQuery {
    entity: Entity,
    transition: &'static Transition,
    depiction: &'static Depiction,
}

#[derive(WorldQuery)]
pub struct TileQuery {
    entity: Entity,
    position: &'static Position,
    parent: &'static Parent,
    children: Option<&'static Children>,
    with_tile: With<Tile>,
}

#[derive(WorldQuery)]
pub struct ZoneQuery {
    entity: Entity,
    zone: &'static Zone,
    with_zone: With<Zone>,
}

#[sysfail(log)]
pub fn enter(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut proxy: EventWriter<ProxyCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(Entity, &Client, Option<&CombatState>, &Parent), With<Online>>,
    transitions: Query<TransitionQuery>,
    tiles: Query<TileQuery>,
    zones: Query<ZoneQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Enter(target) = &command.command {
            let (player, client, combat_state, tile) = players
                .iter_mut()
                .find(|(_, c, _, _)| c.id == command.from)
                .context("Player not found")?;

            if combat_state.is_some() {
                outbox.send_text(client.id, "You can't move while in combat.");

                continue;
            }

            let tile = tiles.get(tile.get())?;

            let transitions_here = match get_transitions(&tile.children, &transitions) {
                Ok(transitions) => transitions,
                Err(err) => {
                    outbox.send_text(client.id, err.to_string());

                    continue;
                }
            };

            let found_transition = match find_transition(target, &transitions_here, &transitions) {
                Ok(transition) => transition,
                Err(err) => {
                    outbox.send_text(client.id, err.to_string());

                    continue;
                }
            };

            match execute_enter(&mut bevy, &player, &tiles, &zones, found_transition) {
                Ok(_) => (),
                Err(err) => {
                    outbox.send_text(client.id, err.to_string());

                    continue;
                }
            }

            proxy.send(ProxyCommand(ParsedCommand {
                from: client.id,
                command: Command::Look(None),
            }));
        }
    }

    Ok(())
}

#[derive(Error, Debug, PartialEq)]
enum GetTransitionsError {
    #[error("There is nowhere to enter from here.")]
    NoTransitions,
}

fn get_transitions(
    siblings: &Option<&Children>,
    transitions: &Query<TransitionQuery>,
) -> Result<Vec<Entity>, GetTransitionsError> {
    let transitions = siblings
        .map(|siblings| {
            siblings
                .iter()
                .filter_map(|child| transitions.get(*child).ok())
                .map(|transition| transition.entity)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if transitions.is_empty() {
        Err(GetTransitionsError::NoTransitions)?
    }

    Ok(transitions)
}

#[derive(Error, Debug, PartialEq)]
enum FindTransitionError {
    #[error("Could not find entrance.")]
    NotFound,
}

fn find_transition<'a>(
    target: &Option<String>,
    transitions: &[Entity],
    query: &'a Query<TransitionQuery>,
) -> Result<&'a Transition, FindTransitionError> {
    transitions
        .iter()
        .find_map(|&transition| {
            let transition = query.get(transition).ok()?;

            match target {
                Some(target_str) if transition.depiction.tags.contains(target_str) => {
                    Some(transition.transition)
                }
                None => Some(transition.transition),
                _ => None,
            }
        })
        .ok_or(FindTransitionError::NotFound)
}

fn execute_enter(
    bevy: &mut Commands,
    player: &Entity,
    tiles: &Query<TileQuery>,
    zones: &Query<ZoneQuery>,
    transition: &Transition,
) -> Result<(), anyhow::Error> {
    let target = tiles
        .iter()
        .find_map(|tile| {
            zones.get(tile.parent.get()).ok().and_then(|zone| {
                if zone.zone.name == transition.zone && tile.position.0 == transition.position {
                    Some(tile.entity)
                } else {
                    None
                }
            })
        })
        .context("Target not found")?;

    bevy.entity(*player).set_parent(target);

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
        transition_builder::TransitionBuilder,
        utils::{get_message_content, send_message},
    };

    use super::*;

    #[test]
    fn parses() {
        let target = handle_enter("enter the void");
        assert_eq!(target, Ok(Command::Enter(Some("the void".into()))));

        let no_target = handle_enter("enter");
        assert_eq!(no_target, Ok(Command::Enter(None)));
    }

    #[test]
    fn enters_by_tag() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, enter);

        let start_zone = ZoneBuilder::new().build(&mut app);
        let start = TileBuilder::new().build(&mut app, start_zone);

        let destination_zone = ZoneBuilder::new().build(&mut app);
        let destination = TileBuilder::new().build(&mut app, destination_zone);

        TransitionBuilder::new()
            .tags(&vec!["the void"])
            .build(&mut app, start, destination);

        let (player, client_id, _) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "enter the void");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), destination);
    }

    #[test]
    fn enters_first_if_no_tag() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, enter);

        let start_zone = ZoneBuilder::new().build(&mut app);
        let start = TileBuilder::new().build(&mut app, start_zone);

        let first_zone = ZoneBuilder::new().build(&mut app);
        let first = TileBuilder::new().build(&mut app, first_zone);

        let second_zone = ZoneBuilder::new().build(&mut app);
        let second = TileBuilder::new().build(&mut app, second_zone);

        TransitionBuilder::new().build(&mut app, start, first);
        TransitionBuilder::new().build(&mut app, start, second);

        let (player, client_id, _) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "enter");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), first);
    }

    #[test]
    fn transition_not_found() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, enter);

        let start_zone = ZoneBuilder::new().build(&mut app);
        let start = TileBuilder::new().build(&mut app, start_zone);

        let other_zone = ZoneBuilder::new().build(&mut app);
        let other = TileBuilder::new().build(&mut app, other_zone);

        TransitionBuilder::new()
            .tags(&vec!["enter the void"])
            .build(&mut app, start, other);

        let (_, client_id, _) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "enter at your own risk");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert!(content.contains("Could not find entrance."));
    }

    #[test]
    fn no_transition() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, enter);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "enter the dragon");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert!(content.contains("There is nowhere to enter from here."));
    }
}
