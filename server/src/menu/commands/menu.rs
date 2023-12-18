use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    input::events::{Command, ParseError, ParsedCommand, ProxyCommand},
    interact::components::{InMenu, Interaction, Interactions, MenuType},
    player::components::{Client, Online},
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_menu(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^.*$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Menu(content.into())),
    }
}

#[derive(WorldQuery)]
pub struct InteractableQuery {
    entity: Entity,
    depiction: &'static Depiction,
    interactions: &'static Interactions,
}

#[sysfail(log)]
pub fn menu(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut proxy: EventWriter<ProxyCommand>,
    players: Query<(Entity, &Client, &InMenu), With<Online>>,
    interactables: Query<InteractableQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Menu(content) = &command.command {
            let (player, client, menu) = players
                .iter()
                .find(|(_, c, _)| c.id == command.from)
                .context("Player not found")?;

            if let Err(err) = execute_interaction(
                &mut bevy,
                &mut proxy,
                player,
                client,
                menu,
                content,
                &interactables,
            ) {
                outbox.send_text(client.id, err.to_string());
            }
        }
    }

    Ok(())
}

#[derive(Error, Debug, PartialEq)]
enum InteractionError {
    #[error("\"{0}\" is not a valid option.")]
    InvalidOption(String),
}

fn execute_interaction(
    bevy: &mut Commands,
    proxy: &mut EventWriter<ProxyCommand>,
    player: Entity,
    client: &Client,
    menu: &InMenu,
    option: &str,
    interactables: &Query<InteractableQuery>,
) -> Result<(), anyhow::Error> {
    let option = option
        .parse::<usize>()
        .map_err(|_| InteractionError::InvalidOption(option.into()))?;

    #[allow(irrefutable_let_patterns)]
    if let MenuType::Examine(entity) = menu.0 {
        let interactable = interactables.get(entity)?;

        let interaction = interactable
            .interactions
            .0
            .iter()
            .filter(|i| i.usable_in_menu())
            .nth(option - 1)
            .ok_or(InteractionError::InvalidOption(option.to_string()))?;

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
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        interact::commands::{examine::*, take::*},
        test::{
            app_builder::AppBuilder,
            item_builder::ItemBuilder,
            player_builder::PlayerBuilder,
            tile_builder::{TileBuilder, ZoneBuilder},
            utils::send_message,
        },
    };

    use super::*;

    #[test]
    fn performs_interaction() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, (menu, examine, take));

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

        send_message(&mut app, client_id, "1");
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
