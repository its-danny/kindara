use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    items::components::{Inventory, Item, Surface},
    npc::components::Npc,
    player::components::{Character, Client, Online},
    spatial::components::Tile,
    value_or_continue,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_scan(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| {
        Regex::new(r"^scan(( (?P<inventory>inventory|inv|i))|( (?P<target>.*)))?$").unwrap()
    });

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let inventory = captures.name("inventory").is_some();

            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Scan((inventory, target)))
        }
    }
}

pub fn scan(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    inventories: Query<(Entity, &Children), With<Inventory>>,
    items: Query<(
        Entity,
        &Item,
        &Depiction,
        Option<&Surface>,
        Option<&Children>,
    )>,
    npcs: Query<(Entity, &Depiction), With<Npc>>,
    players: Query<(Entity, &Client, &Character, &Parent, &Children), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Scan((inventory, target)) = &command.command {
            let (_, client, _, tile, children) =
                value_or_continue!(players.iter().find(|(_, c, _, _, _)| c.id == command.from));

            let entities_to_scan = if *inventory {
                children
                    .iter()
                    .find_map(|child| inventories.get(*child).ok())
                    .map(|(_, inventory)| inventory.iter().collect())
                    .unwrap_or_else(Vec::new)
            } else if let Some(target) = target {
                items
                    .iter()
                    .filter(|(_, _, _, surface, _)| surface.is_some())
                    .find(|(entity, _, depiction, _, _)| depiction.matches_query(entity, target))
                    .and_then(|(_, _, _, _, children)| children)
                    .map(|children| children.iter().collect())
                    .unwrap_or_else(Vec::new)
            } else {
                tiles
                    .get(tile.get())
                    .ok()
                    .map(|siblings| siblings.iter().collect())
                    .unwrap_or_else(Vec::new)
            };

            let output: Vec<String> = entities_to_scan
                .iter()
                .filter_map(|entity| {
                    if let Ok((entity, _, depiction, _, _)) = items.get(**entity) {
                        Some(format!("#{}: {}", entity.index(), depiction.short_name))
                    } else if let Ok((entity, depiction)) = npcs.get(**entity) {
                        Some(format!("#{}: {}", entity.index(), depiction.short_name))
                    } else if let Ok((entity, _, character, _, _)) = players.get(**entity) {
                        Some(format!("#{}: {}", entity.index(), character.name))
                    } else {
                        None
                    }
                })
                .collect();

            if output.is_empty() {
                outbox.send_text(client.id, "Try as you might, you find nothing.");
            } else {
                outbox.send_text(client.id, output.join("\n"));
            }
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
    fn at_tile() {
        let mut app = AppBuilder::new().build();
        app.add_system(scan);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);
        let item = ItemBuilder::new()
            .short_name("rock")
            .tile(tile)
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new()
            .name("Astrid")
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "scan");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            format!("#{}: rock\n#{}: Astrid", item.index(), player.index())
        );
    }

    #[test]
    fn on_surface() {
        let mut app = AppBuilder::new().build();
        app.add_system(scan);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let pile = ItemBuilder::new()
            .name("rock pile")
            .is_surface(SurfaceKind::Floor, 10)
            .tile(tile)
            .build(&mut app);
        let rock = ItemBuilder::new().short_name("rock").build(&mut app);

        app.world.entity_mut(pile).add_child(rock);

        let (_, client_id, _) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "scan rock pile");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, format!("#{}: rock", rock.index()));
    }

    #[test]
    fn in_inventory() {
        let mut app = AppBuilder::new().build();
        app.add_system(scan);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let rock = ItemBuilder::new().short_name("rock").build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        app.world.entity_mut(inventory.unwrap()).add_child(rock);

        send_message(&mut app, client_id, "scan inv");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, format!("#{}: rock", rock.index()));
    }
}
