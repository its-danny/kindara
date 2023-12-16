use std::sync::OnceLock;

use bevy::{
    ecs::query::{QueryEntityError, WorldQuery},
    prelude::*,
};
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    items::components::{Inventory, Item, Surface},
    player::components::{Client, Online},
    spatial::components::Tile,
    value_or_continue,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_place(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| {
        Regex::new(r"^place( (?P<object>.*?))?( (on|against|in))?( (?P<target>.*))?$").unwrap()
    });

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let object = captures
                .name("object")
                .map(|m| m.as_str().trim())
                .ok_or(ParseError::InvalidArguments("Place what?".into()))?;

            let target = captures
                .name("target")
                .map(|m| m.as_str().trim())
                .ok_or(ParseError::InvalidArguments("Place where?".into()))?;

            Ok(Command::Place((object.into(), target.into())))
        }
    }
}

#[derive(WorldQuery)]
pub struct InventoryQuery {
    children: Option<&'static Children>,
    with_inventory: With<Inventory>,
}

#[derive(WorldQuery)]
pub struct ItemQuery {
    entity: Entity,
    item: &'static Item,
    depiction: &'static Depiction,
    interactions: Option<&'static Interactions>,
    children: Option<&'static Children>,
}

pub fn place(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Parent, &Children), With<Online>>,
    inventories: Query<InventoryQuery>,
    tiles: Query<&Children, With<Tile>>,
    items: Query<ItemQuery>,
    surfaces: Query<&Surface>,
) {
    for command in commands.iter() {
        if let Command::Place((object, target)) = &command.command {
            let (client, player_tile, player_children) =
                value_or_continue!(players.iter_mut().find(|(c, _, _)| c.id == command.from));
            let siblings = value_or_continue!(tiles.get(player_tile.get()).ok());
            let inventory = value_or_continue!(player_children
                .iter()
                .find_map(|child| inventories.get(*child).ok()));

            let object = match get_object(object, &inventory, &items) {
                Ok(object) => object,
                Err(err) => {
                    outbox.send_text(client.id, err.to_string());

                    continue;
                }
            };

            let target = match get_target(target, siblings, &items) {
                Ok(target) => target,
                Err(err) => {
                    outbox.send_text(client.id, err.to_string());

                    continue;
                }
            };

            match place_object(
                &mut bevy,
                target,
                object,
                &surfaces.get(target).ok(),
                &items,
            ) {
                Ok(msg) => outbox.send_text(client.id, msg),
                Err(err) => outbox.send_text(client.id, err.to_string()),
            }
        }
    }
}

#[derive(Error, Debug, PartialEq)]
enum ObjectError {
    #[error("You don't have a {0}.")]
    NotFound(String),
    #[error("You can't place the {0}.")]
    NotPlacable(String),
}

fn get_object(
    target: &str,
    inventory: &InventoryQueryItem,
    items: &Query<ItemQuery>,
) -> Result<Entity, ObjectError> {
    let object = inventory
        .children
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| items.get(*child).ok())
        .find(|item| item.depiction.matches_query(&item.entity, target))
        .ok_or(ObjectError::NotFound(target.into()))?;

    if !object
        .interactions
        .map_or(false, |i| i.0.contains(&Interaction::Place))
    {
        return Err(ObjectError::NotPlacable(object.depiction.name.clone()));
    }

    Ok(object.entity)
}

#[derive(Error, Debug, PartialEq)]
enum TargetError {
    #[error("You don't see a {0} here.")]
    NotFound(String),
}

fn get_target(
    target: &str,
    children: &Children,
    items: &Query<ItemQuery>,
) -> Result<Entity, TargetError> {
    let target = children
        .iter()
        .filter_map(|child| items.get(*child).ok())
        .find(|item| item.depiction.matches_query(&item.entity, target))
        .ok_or(TargetError::NotFound(target.into()))?;

    Ok(target.entity)
}

#[derive(Error, Debug, PartialEq)]
enum PlaceError {
    #[error("The {0} is full.")]
    AtCapacity(String),
    #[error("You can't place the {0} on the {1}.")]
    NotPlacable(String, String),
    #[error("Something broke!")]
    QueryEntityError(#[from] QueryEntityError),
}

fn place_object(
    bevy: &mut Commands,
    target: Entity,
    object: Entity,
    surface: &Option<&Surface>,
    items: &Query<ItemQuery>,
) -> Result<String, PlaceError> {
    let Some(surface) = surface else {
        return Err(PlaceError::NotPlacable(
            items.get(object)?.depiction.name.clone(),
            items.get(target)?.depiction.name.clone(),
        ));
    };

    let target = items.get(target)?;
    let object = items.get(object)?;

    if target.children.map_or(false, |children| {
        children
            .iter()
            .filter_map(|child| items.get(*child).ok())
            .map(|item| item.item.size.value())
            .sum::<u8>()
            + object.item.size.value()
            > surface.capacity
    }) {
        return Err(PlaceError::AtCapacity(target.depiction.name.clone()));
    }

    bevy.entity(object.entity).set_parent(target.entity);

    Ok(format!(
        "You place the {} {} the {}.",
        object.depiction.name, surface.kind, target.depiction.name,
    ))
}

#[cfg(test)]
mod tests {
    use crate::{
        items::components::{Size, SurfaceKind},
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
        let object_and_target = handle_place("place rock in pile");
        assert_eq!(
            object_and_target,
            Ok(Command::Place(("rock".into(), "pile".into())))
        );

        let no_object = handle_place("place");
        assert_eq!(
            no_object,
            Err(ParseError::InvalidArguments("Place what?".into()))
        );

        let no_target = handle_place("place rock");
        assert_eq!(
            no_target,
            Err(ParseError::InvalidArguments("Place where?".into()))
        );
    }

    #[test]
    fn place_an_item() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, place);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let table = ItemBuilder::new()
            .name("table")
            .is_surface(SurfaceKind::Floor, 1)
            .tile(tile)
            .build(&mut app);
        let plate = ItemBuilder::new()
            .name("dinner plate")
            .tags(vec!["plate"])
            .interactions(vec![Interaction::Place])
            .build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .has_inventory()
            .tile(tile)
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(plate);

        send_message(&mut app, client_id, "place plate on table");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You place the dinner plate on the table.");
        assert!(app.world.get::<Children>(table).unwrap().contains(&plate));
    }

    #[test]
    fn object_not_found() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, place);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .has_inventory()
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "place plate on table");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You don't have a plate.");
    }

    #[test]
    fn object_not_placable() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, place);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let plate = ItemBuilder::new()
            .name("dinner plate")
            .tags(vec!["plate"])
            .build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .has_inventory()
            .tile(tile)
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(plate);

        send_message(&mut app, client_id, "place plate on table");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You can't place the dinner plate.");
    }

    #[test]
    fn target_not_found() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, place);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let plate = ItemBuilder::new()
            .name("dinner plate")
            .tags(vec!["plate"])
            .interactions(vec![Interaction::Place])
            .build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .has_inventory()
            .tile(tile)
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(plate);

        send_message(&mut app, client_id, "place plate on table");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You don't see a table here.");
    }

    #[test]
    fn target_not_a_surface() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, place);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        ItemBuilder::new().name("table").tile(tile).build(&mut app);
        let plate = ItemBuilder::new()
            .name("dinner plate")
            .tags(vec!["plate"])
            .interactions(vec![Interaction::Place])
            .build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .has_inventory()
            .tile(tile)
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(plate);

        send_message(&mut app, client_id, "place plate on table");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You can't place the dinner plate on the table.");
    }

    #[test]
    fn target_at_capacity() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, place);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let table = ItemBuilder::new()
            .name("table")
            .is_surface(SurfaceKind::Floor, 1)
            .tile(tile)
            .build(&mut app);
        let plate = ItemBuilder::new()
            .name("dinner plate")
            .tags(vec!["plate"])
            .interactions(vec![Interaction::Place])
            .build(&mut app);
        let chair = ItemBuilder::new().size(Size::Medium).build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .has_inventory()
            .tile(tile)
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(plate);
        app.world.entity_mut(table).add_child(chair);

        send_message(&mut app, client_id, "place plate on table");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "The table is full.");
    }
}
