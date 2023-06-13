use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    items::{
        components::{Inventory, Item, Surface},
        utils::item_matches_query,
    },
    player::components::{Client, Online},
    spatial::components::Tile,
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

pub fn place(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Parent, &Children), With<Online>>,
    inventories: Query<Option<&Children>, With<Inventory>>,
    tiles: Query<&Children, With<Tile>>,
    items: Query<(Entity, &Item, Option<&Interactions>, Option<&Children>)>,
    surfaces: Query<&Surface>,
) {
    for command in commands.iter() {
        if let Command::Place((object, target)) = &command.command {
            let Some((client, player_tile, player_children)) = players.iter_mut().find(|(c, _, _)| c.id == command.from) else {
                debug!("Could not find authenticated client: {:?}", command.from);

                continue;
            };

            let Ok(siblings) = tiles.get(player_tile.get()) else {
                debug!("Could not get tile: {:?}", player_tile.get());

                continue;
            };

            let Some(inventory) = player_children.iter().find_map(|child| inventories.get(*child).ok()) else {
                debug!("Could not get inventory for client: {:?}", client);

                continue;
            };

            let Some((object, object_item, object_interactable, _)) = inventory
                .iter()
                .flat_map(|children| children.iter())
                .filter_map(|child| items.get(*child).ok())
                .find(|(entity, item, _, _)| item_matches_query(entity, item, object)) else {
                outbox.send_text(
                    client.id,
                    format!("You don't have a {object}."),
                );

                continue;
            };

            if !object_interactable.map_or(false, |i| i.0.contains(&Interaction::Place)) {
                outbox.send_text(
                    client.id,
                    format!("You can't place the {}.", object_item.name),
                );

                continue;
            }

            let Some((target, target_item, _, target_children)) = siblings
                .iter()
                .filter_map(|child| items.get(*child).ok())
                .find(|(entity, item, _, _)| item_matches_query(entity, item, target)) else {
                outbox.send_text(
                    client.id,
                    format!("You don't see a {target} here."),
                );

                continue;
            };

            let Ok(surface) = surfaces.get(target) else {
                outbox.send_text(
                    client.id,
                    format!("You can't place the {} on the {}.", object_item.name, target_item.name),
                );

                continue;
            };

            if target_children.map_or(false, |children| {
                children
                    .iter()
                    .filter_map(|child| items.get(*child).ok())
                    .map(|(_, item, _, _)| item.size.value())
                    .sum::<u8>()
                    + object_item.size.value()
                    > surface.capacity
            }) {
                outbox.send_text(client.id, format!("The {} is full.", target_item.name));

                continue;
            }

            bevy.entity(object).set_parent(target);

            outbox.send_text(
                client.id,
                format!(
                    "You place the {} {} the {}.",
                    object_item.name, surface.kind, target_item.name,
                ),
            );
        }
    }
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
    fn place_an_item() {
        let mut app = AppBuilder::new().build();
        app.add_system(place);

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

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You place the dinner plate on the table.");
        assert!(app.world.get::<Children>(table).unwrap().contains(&plate));
    }

    #[test]
    fn object_not_found() {
        let mut app = AppBuilder::new().build();
        app.add_system(place);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .has_inventory()
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "place plate on table");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You don't have a plate.");
    }

    #[test]
    fn object_not_placable() {
        let mut app = AppBuilder::new().build();
        app.add_system(place);

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

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You can't place the dinner plate.");
    }

    #[test]
    fn target_not_found() {
        let mut app = AppBuilder::new().build();
        app.add_system(place);

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

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You don't see a table here.");
    }

    #[test]
    fn target_not_a_surface() {
        let mut app = AppBuilder::new().build();
        app.add_system(place);

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

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You can't place the dinner plate on the table.");
    }

    #[test]
    fn target_at_capacity() {
        let mut app = AppBuilder::new().build();
        app.add_system(place);

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

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "The table is full.");
    }
}
