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
    spatial::components::Tile,
    visual::{components::Depiction, utils::name_list},
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_drop(content: &str) -> Result<Command, ParseError> {
    let regex =
        REGEX.get_or_init(|| Regex::new(r"^drop( (?P<all>all))?( (?P<target>.*))?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim())
                .ok_or(ParseError::InvalidArguments("Drop what?".into()))?;

            let all = captures.name("all").is_some();

            Ok(Command::Drop((target.to_string(), all)))
        }
    }
}

#[derive(WorldQuery)]
pub struct ItemQuery {
    entity: Entity,
    depiction: &'static Depiction,
    with_item: With<Item>,
}

#[sysfail(log)]
pub fn drop(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Parent, &Children), With<Online>>,
    inventories: Query<Option<&Children>, With<Inventory>>,
    tiles: Query<Entity, With<Tile>>,
    items: Query<ItemQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Drop((target, all)) = &command.command {
            let (client, tile, children) = players
                .iter_mut()
                .find(|(c, _, _)| c.id == command.from)
                .context("Player not found")?;

            let tile = tiles.get(tile.get())?;

            let items_in_inventory = children
                .iter()
                .find_map(|child| inventories.get(*child).ok())
                .context("Inventory not found")?;

            let mut items_found = search_items(target, &items, &items_in_inventory);

            match drop_item(&mut bevy, target, all, &mut items_found, &items, &tile) {
                Ok(msg) => outbox.send_text(client.id, msg),
                Err(err) => outbox.send_text(client.id, err.to_string()),
            }
        }
    }

    Ok(())
}

fn search_items(
    target: &str,
    items: &Query<ItemQuery>,
    items_in_inventory: &Option<&Children>,
) -> Vec<Entity> {
    items_in_inventory
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|sibling| items.get(*sibling).ok())
        .filter(|item| item.depiction.matches_query(&item.entity, target))
        .map(|item| item.entity)
        .collect()
}

#[derive(Error, Debug, PartialEq)]
enum DropError {
    #[error("You don't have a {0}.")]
    NotFound(String),
}

fn drop_item(
    bevy: &mut Commands,
    target: &str,
    all: &bool,
    items_found: &mut Vec<Entity>,
    items: &Query<ItemQuery>,
    tile: &Entity,
) -> Result<String, DropError> {
    if items_found.is_empty() {
        return Err(DropError::NotFound(target.to_string()));
    }

    if !*all {
        items_found.truncate(1);
    }

    items_found.iter().for_each(|entity| {
        bevy.entity(*entity).set_parent(*tile);
    });

    let item_names = name_list(
        &items_found
            .iter()
            .filter_map(|entity| items.get(*entity).ok())
            .map(|item| item.depiction.name.clone())
            .collect::<Vec<String>>(),
        None,
        true,
    );

    Ok(format!("You drop {item_names}."))
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
        let object = handle_drop("drop rock");
        assert_eq!(object, Ok(Command::Drop(("rock".into(), false))));

        let all = handle_drop("drop all rock");
        assert_eq!(all, Ok(Command::Drop(("rock".into(), true))));

        let no_object = handle_drop("drop");
        assert_eq!(
            no_object,
            Err(ParseError::InvalidArguments("Drop what?".into()))
        );
    }

    #[test]
    fn by_name() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, drop);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let stick = ItemBuilder::new().name("stick").build(&mut app);
        let rock = ItemBuilder::new().name("rock").build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(stick);
        app.world.entity_mut(inventory.unwrap()).add_child(rock);

        send_message(&mut app, client_id, "drop stick");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You drop a stick.");
        assert_eq!(
            app.world
                .get::<Children>(inventory.unwrap())
                .unwrap()
                .contains(&stick),
            false,
        );
    }

    #[test]
    fn by_tag() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, drop);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        let stick = ItemBuilder::new()
            .name("stick")
            .tags(vec!["weapon"])
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(stick);

        send_message(&mut app, client_id, "drop stick");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You drop a stick.");

        assert!(app.world.get::<Children>(inventory.unwrap()).is_none(),);
    }

    #[test]
    fn all() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, drop);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        let stick = ItemBuilder::new().name("stick").build(&mut app);
        app.world.entity_mut(inventory.unwrap()).add_child(stick);

        let another_stick = ItemBuilder::new().name("stick").build(&mut app);
        app.world
            .entity_mut(inventory.unwrap())
            .add_child(another_stick);

        let rock = ItemBuilder::new().name("rock").build(&mut app);
        app.world.entity_mut(inventory.unwrap()).add_child(rock);

        send_message(&mut app, client_id, "drop all stick");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You drop 2 sticks.");

        assert_eq!(
            app.world
                .get::<Children>(inventory.unwrap())
                .unwrap()
                .contains(&stick),
            false
        );

        assert_eq!(
            app.world
                .get::<Children>(inventory.unwrap())
                .unwrap()
                .contains(&another_stick),
            false
        );

        assert!(app
            .world
            .get::<Children>(inventory.unwrap())
            .unwrap()
            .contains(&rock),);
    }

    #[test]
    fn not_found() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, drop);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "drop sword");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You don't have a sword.");

        assert!(app.world.get::<Children>(inventory.unwrap()).is_none());
    }
}
