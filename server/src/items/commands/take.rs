use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use indefinite::indefinite;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    items::{
        components::{Inventory, Item, Surface},
        utils::{item_name_list, item_name_matches},
    },
    player::components::{Client, Online},
    spatial::components::Tile,
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

pub fn take(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Parent, &Children), With<Online>>,
    inventories: Query<Entity, With<Inventory>>,
    tiles: Query<&Children, With<Tile>>,
    items: Query<(Entity, &Item, Option<&Interactions>, Option<&Children>)>,
    surfaces: Query<&Surface>,
) {
    for command in commands.iter() {
        if let Command::Take((target, all, source)) = &command.command {
            let Some((client, tile, children)) = players.iter_mut().find(|(c, _, _)| c.id == command.from) else {
                debug!("Could not find authenticated client: {:?}", command.from);

                continue;
            };

            let Ok(siblings) = tiles.get(tile.get()) else {
                debug!("Could not get tile: {:?}", tile.get());

                continue;
            };

            let Some(inventory) = children.iter().find_map(|child| inventories.get(*child).ok()) else {
                debug!("Could not get inventory for client: {:?}", client);

                continue;
            };

            let to_search = if let Some(source) = source {
                siblings
                    .iter()
                    .filter_map(|sibling| items.get(*sibling).ok())
                    .find(|(sibling, item, _, _)| {
                        surfaces.get(*sibling).is_ok() && item_name_matches(item, source)
                    })
                    .and_then(|(_, _, _, children)| children)
                    .map(|children| {
                        children
                            .iter()
                            .filter_map(|child| items.get(*child).ok())
                            .collect()
                    })
                    .unwrap_or_else(Vec::new)
            } else {
                siblings
                    .iter()
                    .filter_map(|sibling| items.get(*sibling).ok())
                    .collect()
            };

            let mut items_found = to_search
                .iter()
                .filter(|(_, item, _, _)| item_name_matches(item, target))
                .collect::<Vec<_>>();

            if items_found.is_empty() {
                let target = if let Some(source) = source {
                    source
                } else {
                    target
                };

                outbox.send_text(
                    client.id,
                    format!("You don't see {} here.", indefinite(target)),
                );

                continue;
            }

            if items_found.iter().any(|(_, _, interactable, _)| {
                interactable.map_or(true, |i| !i.0.contains(&Interaction::Take))
            }) {
                outbox.send_text(client.id, "You can't take that.");

                continue;
            }

            if !*all {
                items_found.truncate(1);
            }

            items_found.iter().for_each(|(entity, _, _, _)| {
                bevy.entity(*entity).set_parent(inventory);
            });

            let item_names = item_name_list(
                &items_found
                    .iter()
                    .map(|(_, item, _, _)| item.name.clone())
                    .collect::<Vec<String>>(),
            );

            outbox.send_text(client.id, format!("You take {item_names}."));
        }
    }
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
    fn by_name() {
        let mut app = AppBuilder::new().build();
        app.add_system(take);

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

        let content = get_message_content(&mut app, client_id);

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
        app.add_system(take);

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

        let content = get_message_content(&mut app, client_id);

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
        app.add_system(take);

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

        let content = get_message_content(&mut app, client_id);

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
        app.add_system(take);

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

        let content = get_message_content(&mut app, client_id);

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
        app.add_system(take);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "take sword");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "You don't see a sword here.");

        assert!(app.world.get::<Children>(inventory.unwrap()).is_none());
    }
}
