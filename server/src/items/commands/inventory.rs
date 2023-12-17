use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    items::components::{Inventory, Item},
    player::components::{Client, Online},
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_inventory(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(inventory|inv|i)$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Inventory),
    }
}

#[derive(WorldQuery)]
pub struct ItemQuery {
    entity: Entity,
    depiction: &'static Depiction,
    with_item: With<Item>,
}

#[sysfail(log)]
pub fn inventory(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Children), With<Online>>,
    inventories: Query<Option<&Children>, With<Inventory>>,
    items: Query<ItemQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Inventory = &command.command {
            let (client, children) = players
                .iter_mut()
                .find(|(c, _)| c.id == command.from)
                .context("Player not found")?;

            let inventory = children
                .iter()
                .find_map(|child| inventories.get(*child).ok())
                .context("Inventory not found")?;

            let in_inventory = match get_items(&inventory, &items) {
                Ok(items) => items,
                Err(err) => {
                    outbox.send_text(client.id, err.to_string());

                    continue;
                }
            };

            outbox.send_text(client.id, list_items(in_inventory, &items));
        }
    }

    Ok(())
}

#[derive(Error, Debug, PartialEq)]
enum InventoryError {
    #[error("You are not carrying anything.")]
    NotCarryingAnything,
}

fn get_items(
    inventory: &Option<&Children>,
    items: &Query<ItemQuery>,
) -> Result<Vec<Entity>, InventoryError> {
    let mut in_inventory = inventory
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| items.get(*child).ok())
        .collect::<Vec<_>>();

    if in_inventory.is_empty() {
        return Err(InventoryError::NotCarryingAnything);
    }

    in_inventory.sort_by(|a, b| a.depiction.name.cmp(&b.depiction.name));

    Ok(in_inventory.iter().map(|item| item.entity).collect())
}

fn list_items(in_inventory: Vec<Entity>, items: &Query<ItemQuery>) -> String {
    let names = in_inventory
        .iter()
        .filter_map(|item| items.get(*item).ok())
        .map(|item| item.depiction.name.clone())
        .collect::<Vec<_>>()
        .join(", ");

    format!("You are carrying: {names}")
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
    fn carrying_nothing() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, inventory);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "inventory");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You are not carrying anything.");
    }

    #[test]
    fn carrying_an_item() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, inventory);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let item = ItemBuilder::new().name("stick").build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        app.world.entity_mut(item).set_parent(inventory.unwrap());

        send_message(&mut app, client_id, "inventory");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You are carrying: stick");
    }
}
