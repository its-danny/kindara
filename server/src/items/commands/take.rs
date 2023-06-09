use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    items::components::{Inventory, Item},
    player::components::{Client, Online},
    spatial::components::Tile,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_take(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(take|get) (?P<target>.+)$").unwrap());

    if let Some(captures) = regex.captures(content) {
        let target = captures
            .name("target")
            .map(|m| m.as_str().trim().to_lowercase())
            .unwrap_or_default();

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Take(target),
        });

        true
    } else {
        false
    }
}

pub fn take(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Parent, &Children), With<Online>>,
    inventories: Query<Entity, With<Inventory>>,
    tiles: Query<&Children, With<Tile>>,
    items: Query<(Entity, &Item)>,
) {
    for command in commands.iter() {
        if let Command::Take(target) = &command.command {
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

            let Some((entity, item)) = siblings.iter()
                .filter_map(|sibling| items.get(*sibling).ok())
                .find(|(_, item)| item.name.to_lowercase() == target.to_lowercase() || item.tags.contains(&target.to_lowercase()))
            else {
                outbox.send_text(client.id, format!("You don't see a {} here.", target));

                continue;
            };

            bevy.entity(entity).set_parent(inventory);

            outbox.send_text(client.id, format!("You take the {}.", item.name));
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
        app.add_system(take);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        ItemBuilder::new().name("stick").tile(tile).build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory(true)
            .build(&mut app);

        send_message(&mut app, client_id, "take stick");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, format!("You take the stick."));

        assert_eq!(
            app.world.get::<Children>(inventory.unwrap()).unwrap().len(),
            1
        );
    }

    #[test]
    fn by_tag() {
        let mut app = AppBuilder::new().build();
        app.add_system(take);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        ItemBuilder::new()
            .name("stick")
            .tags(vec!["weapon"])
            .tile(tile)
            .build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory(true)
            .build(&mut app);

        send_message(&mut app, client_id, "take weapon");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, format!("You take the stick."));

        assert_eq!(
            app.world.get::<Children>(inventory.unwrap()).unwrap().len(),
            1
        );
    }

    #[test]
    fn not_found() {
        let mut app = AppBuilder::new().build();
        app.add_system(take);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory(true)
            .build(&mut app);

        send_message(&mut app, client_id, "take sword");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, format!("You don't see a sword here."));

        assert!(app.world.get::<Children>(inventory.unwrap()).is_none());
    }
}
