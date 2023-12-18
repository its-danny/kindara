use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{InMenu, Interaction, Interactions, MenuType},
    player::components::{Client, Online},
    spatial::components::Tile,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_examine(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(examine|ex)( (?P<target>.*))?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase())
                .filter(|m| !m.is_empty())
                .ok_or(ParseError::InvalidArguments("Examine what?".into()))?;

            Ok(Command::Examine(target))
        }
    }
}

#[derive(WorldQuery)]
pub struct InteractableQuery {
    entity: Entity,
    depiction: &'static Depiction,
    interactions: Option<&'static Interactions>,
}

#[sysfail(log)]
pub fn examine(
    interactables: Query<InteractableQuery>,
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(Entity, &Client, &Parent), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Examine(target) = &command.command {
            let (player, client, tile) = players
                .iter()
                .find(|(_, c, _)| c.id == command.from)
                .context("Player not found")?;

            let siblings = tiles.get(tile.get())?;

            match execute_examine(&mut bevy, player, target, siblings, &interactables) {
                Ok(msg) => outbox.send_text(client.id, msg),
                Err(err) => outbox.send_text(client.id, err.to_string()),
            }
        }
    }

    Ok(())
}

#[derive(Error, Debug, PartialEq)]
enum ExamineError {
    #[error("You don't see a {0} here")]
    NotFound(#[from] TargetError),
}

fn execute_examine(
    bevy: &mut Commands,
    player: Entity,
    target: &str,
    siblings: &Children,
    interactables: &Query<InteractableQuery>,
) -> Result<String, anyhow::Error> {
    let interactable = get_target(target, siblings, interactables)?;
    let interactable = interactables.get(interactable)?;

    if let Some(interactions) = interactable.interactions {
        let opts = get_interactions(&interactions.0);

        bevy.entity(player)
            .insert(InMenu(MenuType::Examine(interactable.entity)));

        Ok(format!(
            "After thorough inspection, you find you are able to do the following:\n\n{}\n\nType \"quit\" to leave menu.",
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
    NotFound(String),
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
        .ok_or(TargetError::NotFound(target.into()))
}

fn get_interactions(interactions: &[Interaction]) -> Vec<String> {
    interactions
        .iter()
        .enumerate()
        .filter(|(_, i)| i.usable_in_menu())
        .map(|(idx, int)| format!("[{}] {int}", idx + 1))
        .collect()
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
        let target = handle_examine("examine rock");
        assert_eq!(target, Ok(Command::Examine("rock".into())));

        let no_target = handle_examine("examine");
        assert_eq!(
            no_target,
            Err(ParseError::InvalidArguments("Examine what?".into()))
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
            "After thorough inspection, you find you are able to do the following:\n\n[1] Take\n\nType \"quit\" to leave menu."
        );
    }
}
