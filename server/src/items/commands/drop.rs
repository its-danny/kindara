use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    items::{
        components::{Inventory, Item},
        utils::{item_matches_query, item_name_list},
    },
    player::components::{Client, Online},
    spatial::components::Tile,
    value_or_continue,
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

pub fn drop(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Parent, &Children), With<Online>>,
    inventories: Query<Option<&Children>, With<Inventory>>,
    tiles: Query<Entity, With<Tile>>,
    items: Query<(Entity, &Item)>,
) {
    for command in commands.iter() {
        if let Command::Drop((target, all)) = &command.command {
            let (client, tile, children) =
                value_or_continue!(players.iter_mut().find(|(c, _, _)| c.id == command.from));
            let tile = value_or_continue!(tiles.get(tile.get()).ok());
            let items_in_inventory = value_or_continue!(children
                .iter()
                .find_map(|child| inventories.get(*child).ok()));

            let mut items_found = items_in_inventory
                .iter()
                .flat_map(|children| children.iter())
                .filter_map(|sibling| items.get(*sibling).ok())
                .filter(|(entity, item)| item_matches_query(entity, item, target))
                .collect::<Vec<(Entity, &Item)>>();

            if !*all {
                items_found.truncate(1);
            }

            items_found.iter().for_each(|(entity, _)| {
                bevy.entity(*entity).set_parent(tile);
            });

            let item_names = item_name_list(
                &items_found
                    .iter()
                    .map(|(_, item)| item.name.clone())
                    .collect::<Vec<String>>(),
            );

            if item_names.is_empty() {
                outbox.send_text(client.id, format!("You don't have a {target}."));
            } else {
                outbox.send_text(client.id, format!("You drop {item_names}."));
            }
        }
    }
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
    fn by_name() {
        let mut app = AppBuilder::new().build();
        app.add_system(drop);

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

        let content = get_message_content(&mut app, client_id);

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
        app.add_system(drop);

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

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You drop a stick.");

        assert!(app.world.get::<Children>(inventory.unwrap()).is_none(),);
    }

    #[test]
    fn all() {
        let mut app = AppBuilder::new().build();
        app.add_system(drop);

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

        let content = get_message_content(&mut app, client_id);

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
        app.add_system(drop);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "drop sword");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You don't have a sword.");

        assert!(app.world.get::<Children>(inventory.unwrap()).is_none());
    }
}
