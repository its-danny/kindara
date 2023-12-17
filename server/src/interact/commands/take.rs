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
    visual::{components::Depiction, utils::name_list},
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_take(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| {
        Regex::new(r"^(take|get)( (?P<all>all))?( (?P<target>.*?))?( (from))?( (?P<source>.*))?$")
            .unwrap()
    });

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim())
                .ok_or(ParseError::InvalidArguments("Take what?".into()))?;

            let all = captures.name("all").is_some();

            let source = captures
                .name("source")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Take((target.into(), all, source)))
        }
    }
}

#[derive(WorldQuery)]
pub struct ItemQuery {
    entity: Entity,
    depiction: &'static Depiction,
    interactions: Option<&'static Interactions>,
    children: Option<&'static Children>,
    with_item: With<Item>,
}

#[derive(WorldQuery)]
pub struct SurfaceQuery {
    entity: Entity,
    surface: &'static Surface,
}

pub fn take(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Parent, &Children), With<Online>>,
    inventories: Query<Entity, With<Inventory>>,
    tiles: Query<&Children, With<Tile>>,
    items: Query<ItemQuery>,
    surfaces: Query<SurfaceQuery>,
) {
    for command in commands.iter() {
        if let Command::Take((target, all, source)) = &command.command {
            let (client, tile, children) =
                value_or_continue!(players.iter_mut().find(|(c, _, _)| c.id == command.from));
            let siblings = value_or_continue!(tiles.get(tile.get()).ok());
            let inventory = value_or_continue!(children
                .iter()
                .find_map(|child| inventories.get(*child).ok()));

            let to_search = get_searchable_items(siblings, source, &surfaces, &items);
            let mut items_found = search_items(target, &to_search, &items);

            match take_item(
                &mut bevy,
                target,
                all,
                source,
                &mut items_found,
                &items,
                inventory,
            ) {
                Ok(msg) => outbox.send_text(client.id, msg),
                Err(err) => outbox.send_text(client.id, err.to_string()),
            }
        }
    }
}

fn get_searchable_items(
    siblings: &Children,
    source: &Option<String>,
    surfaces: &Query<SurfaceQuery>,
    items: &Query<ItemQuery>,
) -> Vec<Entity> {
    if let Some(source) = source {
        // Find the specific surface item by source and return its children.
        siblings
            .iter()
            .filter_map(|sibling| items.get(*sibling).ok())
            .find(|item| {
                surfaces.get(item.entity).is_ok()
                    && item.depiction.matches_query(&item.entity, source)
            })
            .and_then(|item| item.children)
            .map_or_else(Vec::new, |children| children.iter().copied().collect())
    } else {
        // Return all entities in siblings.j
        siblings
            .iter()
            .filter_map(|sibling| items.get(*sibling).ok())
            .map(|item| item.entity)
            .collect()
    }
}

fn search_items(target: &str, to_search: &[Entity], items: &Query<ItemQuery>) -> Vec<Entity> {
    to_search
        .iter()
        .filter_map(|entity| items.get(*entity).ok())
        .filter(|item| item.depiction.matches_query(&item.entity, target))
        .map(|item| item.entity)
        .collect()
}

#[derive(Error, Debug, PartialEq)]
enum TakeError {
    #[error("You don't see a {0} here.")]
    NotFound(String),
    #[error("You can't take that.")]
    NotTakeable(#[from] ValidateError),
    #[error("Something broke!")]
    QueryEntityError(#[from] QueryEntityError),
}

fn take_item(
    bevy: &mut Commands,
    target: &str,
    all: &bool,
    source: &Option<String>,
    items_found: &mut Vec<Entity>,
    items: &Query<ItemQuery>,
    inventory: Entity,
) -> Result<String, TakeError> {
    if items_found.is_empty() {
        let target = source.as_deref().unwrap_or(target);
        return Err(TakeError::NotFound(target.into()));
    }

    validate_items(items_found, items)?;

    if !*all {
        items_found.truncate(1);
    }

    for &item in items_found.iter() {
        bevy.entity(item).set_parent(inventory);
    }

    let item_names = items_found
        .iter()
        .filter_map(|&item| items.get(item).ok().map(|item| item.depiction.name.clone()))
        .collect::<Vec<_>>();

    let item_names = name_list(&item_names, None, true);

    Ok(format!("You take {item_names}."))
}

#[derive(Error, Debug, PartialEq)]
enum ValidateError {
    #[error("You can't take that.")]
    NotTakeable,
    #[error("Something broke!")]
    QueryEntityError(#[from] QueryEntityError),
}

fn validate_items(items_found: &[Entity], items: &Query<ItemQuery>) -> Result<(), ValidateError> {
    items_found.iter().try_for_each(|&item| {
        let item = items.get(item)?;

        if item
            .interactions
            .map_or(false, |i| i.0.contains(&Interaction::Take))
        {
            Ok(())
        } else {
            Err(ValidateError::NotTakeable)
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::{
        items::components::SurfaceKind,
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
        let object = handle_take("take rock");
        assert_eq!(object, Ok(Command::Take(("rock".into(), false, None))));

        let all = handle_take("take all rock");
        assert_eq!(all, Ok(Command::Take(("rock".into(), true, None))));

        let object_and_target = handle_take("take rock from pile");
        assert_eq!(
            object_and_target,
            Ok(Command::Take(("rock".into(), false, Some("pile".into()))))
        );

        let no_object = handle_take("take");
        assert_eq!(
            no_object,
            Err(ParseError::InvalidArguments("Take what?".into()))
        );
    }

    #[test]
    fn by_name() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, take);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let stick = ItemBuilder::new()
            .name("stick")
            .interactions(vec![Interaction::Take])
            .tile(tile)
            .build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "take stick");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You take a stick.");

        assert!(app
            .world
            .get::<Children>(inventory.unwrap())
            .unwrap()
            .contains(&stick),);
    }

    #[test]
    fn by_tag() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, take);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let stick = ItemBuilder::new()
            .name("stick")
            .tags(vec!["weapon"])
            .interactions(vec![Interaction::Take])
            .tile(tile)
            .build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "take weapon");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You take a stick.");

        assert!(app
            .world
            .get::<Children>(inventory.unwrap())
            .unwrap()
            .contains(&stick),);
    }

    #[test]
    fn all() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, take);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let stick = ItemBuilder::new()
            .name("stick")
            .interactions(vec![Interaction::Take])
            .tile(tile)
            .build(&mut app);

        let another_stick = ItemBuilder::new()
            .name("stick")
            .interactions(vec![Interaction::Take])
            .tile(tile)
            .build(&mut app);

        ItemBuilder::new()
            .name("rock")
            .interactions(vec![Interaction::Take])
            .tile(tile)
            .build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "take all stick");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You take 2 sticks.");

        assert!(app
            .world
            .get::<Children>(inventory.unwrap())
            .unwrap()
            .contains(&stick),);

        assert!(app
            .world
            .get::<Children>(inventory.unwrap())
            .unwrap()
            .contains(&another_stick),);
    }

    #[test]
    fn from_another() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, take);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let table = ItemBuilder::new()
            .name("table")
            .is_surface(SurfaceKind::Floor, 1)
            .tile(tile)
            .build(&mut app);

        let plate = ItemBuilder::new()
            .name("plate")
            .interactions(vec![Interaction::Take])
            .build(&mut app);

        app.world.entity_mut(table).add_child(plate);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "take plate from table");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!("You take a plate.", content);

        assert!(app.world.get::<Children>(table).is_none());
        assert!(app
            .world
            .get::<Children>(inventory.unwrap())
            .unwrap()
            .contains(&plate),);
    }

    #[test]
    fn not_found() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, take);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "take sword");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You don't see a sword here.");

        assert!(app.world.get::<Children>(inventory.unwrap()).is_none());
    }
}
