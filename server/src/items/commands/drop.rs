use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    items::{
        components::{Inventory, Item},
        utils::item_name_list,
    },
    player::components::{Client, Online},
    spatial::components::Tile,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_drop(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(|| Regex::new(r"^drop ((?P<all>all) )?(?P<target>.+)$").unwrap());

    if let Some(captures) = regex.captures(content) {
        let target = captures
            .name("target")
            .map(|m| m.as_str().trim().to_lowercase())
            .unwrap_or_default();

        let all = captures.name("all").is_some();

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Drop((target, all)),
        });

        true
    } else {
        false
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
            let Some((client, tile, children)) = players.iter_mut().find(|(c, _, _)| c.id == command.from) else {
                debug!("Could not find authenticated client: {:?}", command.from);

                continue;
            };

            let Ok(tile) = tiles.get(tile.get()) else {
                debug!("Could not get tile: {:?}", tile.get());

                continue;
            };

            let Some(items_in_inventory) = children.iter().find_map(|child| inventories.get(*child).ok()) else {
                debug!("Could not get inventory for client: {:?}", client);

                continue;
            };

            let mut items_found = items_in_inventory
                .iter()
                .flat_map(|children| children.iter())
                .filter_map(|sibling| items.get(*sibling).ok())
                .filter(|(_, item)| {
                    item.name.to_lowercase() == target.to_lowercase()
                        || item.name_on_ground.to_lowercase() == target.to_lowercase()
                        || item.tags.contains(&target.to_lowercase())
                })
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
            .has_inventory(true)
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(stick);
        app.world.entity_mut(inventory.unwrap()).add_child(rock);

        send_message(&mut app, client_id, "drop stick");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, format!("You drop a stick."));

        assert_eq!(
            app.world.get::<Children>(inventory.unwrap()).unwrap().len(),
            1
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
            .has_inventory(true)
            .build(&mut app);

        let stick = ItemBuilder::new()
            .name("stick")
            .tags(vec!["weapon"])
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(stick);

        send_message(&mut app, client_id, "drop stick");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, format!("You drop a stick."));

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
            .has_inventory(true)
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

        assert_eq!(content, format!("You drop 2 sticks."));

        assert_eq!(
            app.world.get::<Children>(inventory.unwrap()).unwrap().len(),
            1
        );
    }

    #[test]
    fn not_found() {
        let mut app = AppBuilder::new().build();
        app.add_system(drop);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory(true)
            .build(&mut app);

        send_message(&mut app, client_id, "drop sword");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, format!("You don't have a sword."));

        assert!(app.world.get::<Children>(inventory.unwrap()).is_none());
    }
}
