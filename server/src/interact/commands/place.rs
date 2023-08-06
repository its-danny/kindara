use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

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

pub fn place(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Parent, &Children), With<Online>>,
    inventories: Query<Option<&Children>, With<Inventory>>,
    tiles: Query<&Children, With<Tile>>,
    items: Query<(
        Entity,
        &Item,
        &Depiction,
        Option<&Interactions>,
        Option<&Children>,
    )>,
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

            let Some((object, object_item, object_depiction, object_interactable, _)) = inventory
                .iter()
                .flat_map(|children| children.iter())
                .filter_map(|child| items.get(*child).ok())
                .find(|(e, _, d, _, _)| d.matches_query(e, object)) else {
                outbox.send_text(
                    client.id,
                    format!("You don't have a {object}."),
                );

                continue;
            };

            if !object_interactable.map_or(false, |i| i.0.contains(&Interaction::Place)) {
                outbox.send_text(
                    client.id,
                    format!("You can't place the {}.", object_depiction.name),
                );

                continue;
            }

            let Some((target, _, target_depiction, _, target_children)) = siblings
                .iter()
                .filter_map(|child| items.get(*child).ok())
                .find(|(e, _,d,  _, _)| d.matches_query(e, target)) else {
                outbox.send_text(
                    client.id,
                    format!("You don't see a {target} here."),
                );

                continue;
            };

            let Ok(surface) = surfaces.get(target) else {
                outbox.send_text(
                    client.id,
                    format!("You can't place the {} on the {}.", object_depiction.name, target_depiction.name),
                );

                continue;
            };

            if target_children.map_or(false, |children| {
                children
                    .iter()
                    .filter_map(|child| items.get(*child).ok())
                    .map(|(_, item, _, _, _)| item.size.value())
                    .sum::<u8>()
                    + object_item.size.value()
                    > surface.capacity
            }) {
                outbox.send_text(client.id, format!("The {} is full.", target_depiction.name));

                continue;
            }

            bevy.entity(object).set_parent(target);

            outbox.send_text(
                client.id,
                format!(
                    "You place the {} {} the {}.",
                    object_depiction.name, surface.kind, target_depiction.name,
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
