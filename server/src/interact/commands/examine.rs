use std::sync::OnceLock;

use bevy::{
    ecs::query::{QueryEntityError, WorldQuery},
    prelude::*,
};
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    input::events::{Command, ParseError, ParsedCommand, ProxyCommand},
    interact::components::{InMenu, Interaction, Interactions, MenuType},
    player::components::{Client, Online},
    spatial::components::Tile,
    value_or_continue,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_examine(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| {
        Regex::new(r"^(examine|ex)(?: (?P<select>do)(?: (?P<option>\d+))?)?(?: (?P<target>.*))?$")
            .unwrap()
    });

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let select = captures
                .name("select")
                .map(|m| m.as_str().trim().to_lowercase());

            let option = captures
                .name("option")
                .and_then(|m| m.as_str().parse::<usize>().ok());

            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            match (select, option, target) {
                (None, None, None) => Err(ParseError::InvalidArguments("Examine what?".into())),
                (Some(_), None, None) => Err(ParseError::InvalidArguments("Do what?".into())),
                (None, None, Some(target)) => Ok(Command::Examine((Some(target), None))),
                (Some(_), Some(option), None) => Ok(Command::Examine((None, Some(option)))),
                _ => Err(ParseError::WrongCommand),
            }
        }
    }
}

#[derive(WorldQuery)]
pub struct InteractableQuery {
    entity: Entity,
    depiction: &'static Depiction,
    interactions: Option<&'static Interactions>,
}

pub fn examine(
    interactables: Query<InteractableQuery>,
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut proxy: EventWriter<ProxyCommand>,
    players: Query<(Entity, &Client, &Parent, Option<&InMenu>), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Examine((target, option)) = &command.command {
            let (player, client, tile, in_menu) =
                value_or_continue!(players.iter().find(|(_, c, _, _)| c.id == command.from));
            let siblings = value_or_continue!(tiles.get(tile.get()).ok());

            if let Some(target) = target {
                match execute_examine(&mut bevy, player, target, siblings, &interactables) {
                    Ok(msg) => outbox.send_text(client.id, msg),
                    Err(err) => outbox.send_text(client.id, err.to_string()),
                }
            }

            if let Some(option) = option {
                match execute_interaction(
                    &mut bevy,
                    &mut proxy,
                    player,
                    client,
                    &in_menu,
                    option,
                    &interactables,
                ) {
                    Ok(_) => {}
                    Err(err) => outbox.send_text(client.id, err.to_string()),
                }
            }
        }
    }
}

#[derive(Error, Debug, PartialEq)]
enum ExamineError {
    #[error("You don't see a {0} here")]
    NoTarget(#[from] TargetError),
    #[error("Something broke!")]
    QueryEntityError(#[from] QueryEntityError),
}

fn execute_examine(
    bevy: &mut Commands,
    player: Entity,
    target: &str,
    siblings: &Children,
    interactables: &Query<InteractableQuery>,
) -> Result<String, ExamineError> {
    let interactable = get_target(target, siblings, interactables)?;
    let interactable = interactables.get(interactable)?;

    if let Some(interactions) = interactable.interactions {
        let opts = get_interactions(&interactions.0);

        bevy.entity(player)
            .insert(InMenu(MenuType::Examine(interactable.entity)));

        Ok(format!(
            "After thorough inspection, you find you are able to do the following:\n\n{}",
            opts.join(", ")
        ))
    } else {
        Ok(format!(
            "{} has no interactions.",
            interactable.depiction.name
        ))
    }
}

#[derive(Error, Debug, PartialEq)]
enum TargetError {
    #[error("You don't see a {0} here")]
    NoTarget(String),
}

fn get_target(
    target: &str,
    siblings: &Children,
    interactables: &Query<InteractableQuery>,
) -> Result<Entity, TargetError> {
    siblings
        .iter()
        .filter_map(|sibling| interactables.get(*sibling).ok())
        .find(|i| i.depiction.matches_query(&i.entity, target))
        .map(|i| i.entity)
        .ok_or(TargetError::NoTarget(target.into()))
}

fn get_interactions(interactions: &[Interaction]) -> Vec<String> {
    interactions
        .iter()
        .enumerate()
        .filter(|(_, i)| i.usable_in_menu())
        .map(|(idx, int)| format!("[{}] {int}", idx + 1))
        .collect()
}

#[derive(Error, Debug, PartialEq)]
enum InteractionError {
    #[error("You are not in a menu.")]
    NotInMenu,
    #[error("{0} has no interactions.")]
    NoInteractions(String),
    #[error("Incorrect option.")]
    IncorrectOption,
    #[error("Something broke!")]
    QueryEntityError(#[from] QueryEntityError),
}

fn execute_interaction(
    bevy: &mut Commands,
    proxy: &mut EventWriter<ProxyCommand>,
    player: Entity,
    client: &Client,
    in_menu: &Option<&InMenu>,
    option: &usize,
    interactables: &Query<InteractableQuery>,
) -> Result<(), InteractionError> {
    let menu = in_menu.ok_or(InteractionError::NotInMenu)?;

    #[allow(irrefutable_let_patterns)]
    if let MenuType::Examine(entity) = menu.0 {
        let interactable = interactables.get(entity)?;

        if let Some(interactions) = interactable.interactions {
            let interaction = interactions
                .0
                .iter()
                .filter(|i| i.usable_in_menu())
                .nth(option - 1)
                .ok_or(InteractionError::IncorrectOption)?;

            match interaction {
                Interaction::Sit => proxy.send(ProxyCommand(ParsedCommand {
                    from: client.id,
                    command: Command::Sit(Some(interactable.depiction.name.clone())),
                })),
                Interaction::Take => proxy.send(ProxyCommand(ParsedCommand {
                    from: client.id,
                    command: Command::Take((interactable.depiction.name.clone(), false, None)),
                })),
                _ => debug!("Unhandled interaction: {:?}", interaction),
            }

            bevy.entity(player).remove::<InMenu>();
        } else {
            return Err(InteractionError::NoInteractions(
                interactable.depiction.name.clone(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        interact::commands::take::*,
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
        let target = handle_examine("examine rock");
        assert_eq!(target, Ok(Command::Examine((Some("rock".into()), None))));

        let no_target = handle_examine("examine");
        assert_eq!(
            no_target,
            Err(ParseError::InvalidArguments("Examine what?".into()))
        );

        let option = handle_examine("examine do 1");
        assert_eq!(option, Ok(Command::Examine((None, Some(1)))));

        let no_option = handle_examine("examine do");
        assert_eq!(
            no_option,
            Err(ParseError::InvalidArguments("Do what?".into()))
        );
    }

    #[test]
    fn lists_interactions() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, examine);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        ItemBuilder::new()
            .name("Rock")
            .short_name("rock")
            .interactions(vec![Interaction::Take])
            .tile(tile)
            .build(&mut app);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "examine rock");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            "After thorough inspection, you find you are able to do the following:\n\n[1] Take"
        );
    }

    #[test]
    fn performs_interaction() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, (examine, take));

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let item = ItemBuilder::new()
            .name("Rock")
            .short_name("rock")
            .interactions(vec![Interaction::Take])
            .tile(tile)
            .build(&mut app);

        let (player, client_id, inventory) = PlayerBuilder::new()
            .has_inventory()
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "examine rock");
        app.update();

        assert!(app.world.get::<InMenu>(player).is_some());

        send_message(&mut app, client_id, "examine do 1");
        app.update();

        // Second update to process the ProxyCommand event
        app.update();

        assert!(app
            .world
            .get::<Children>(inventory.unwrap())
            .unwrap()
            .contains(&item));
    }
}
