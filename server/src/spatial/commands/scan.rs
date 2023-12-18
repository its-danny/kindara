use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    items::components::{Inventory, Item, Surface},
    npc::components::Npc,
    player::components::{Character, Client, Online},
    spatial::components::Tile,
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

#[derive(WorldQuery)]
pub struct InventoryQuery {
    entity: Entity,
    children: &'static Children,
    with_inventory: With<Inventory>,
}

#[derive(WorldQuery)]
pub struct ItemQuery {
    entity: Entity,
    item: &'static Item,
    depiction: &'static Depiction,
    surface: Option<&'static Surface>,
    children: Option<&'static Children>,
}

#[derive(WorldQuery)]
pub struct NpcQuery {
    entity: Entity,
    depiction: &'static Depiction,
    with_npc: With<Npc>,
}

#[derive(WorldQuery)]
pub struct PlayerQuery {
    entity: Entity,
    client: &'static Client,
    character: &'static Character,
    parent: &'static Parent,
    children: &'static Children,
    with_online: With<Online>,
}

#[derive(WorldQuery)]
pub struct TileQuery {
    children: &'static Children,
    with_tile: With<Tile>,
}

#[sysfail(log)]
pub fn scan(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    inventories: Query<InventoryQuery>,
    items: Query<ItemQuery>,
    npcs: Query<NpcQuery>,
    players: Query<PlayerQuery>,
    tiles: Query<TileQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Scan((inventory, target)) = &command.command {
            let player = players
                .iter()
                .find(|player| player.client.id == command.from)
                .context("Player not found")?;

            let entities_to_scan = collect_entities_to_scan(
                target,
                inventory,
                player.parent,
                player.children,
                &inventories,
                &items,
                &tiles,
            );

            let output: Vec<String> = if entities_to_scan.is_empty() {
                vec!["Try as you might, you find nothing.".to_string()]
            } else {
                entities_to_scan
                    .iter()
                    .filter_map(|&entity| {
                        format_entity_description(entity, &items, &npcs, &players)
                    })
                    .collect()
            };

            outbox.send_text(player.client.id, output.join("\n"));
        }
    }

    Ok(())
}

fn collect_entities_to_scan(
    target: &Option<String>,
    in_inventory: &bool,
    tile: &Parent,
    children: &Children,
    inventories: &Query<InventoryQuery>,
    items: &Query<ItemQuery>,
    tiles: &Query<TileQuery>,
) -> Vec<Entity> {
    // Scan player's inventory
    if *in_inventory {
        return children
            .iter()
            .find_map(|child| inventories.get(*child).ok())
            .map(|inventory| inventory.children.iter().copied().collect())
            .unwrap_or_default();
    }

    // Scan for a specific target
    if let Some(target_name) = target {
        return items
            .iter()
            .filter(|item| item.surface.is_some())
            .find(|item| item.depiction.matches_query(&item.entity, target_name))
            .and_then(|item| item.children)
            .map(|children| children.iter().copied().collect())
            .unwrap_or_else(Vec::new);
    }

    // Scan the current tile
    tiles
        .get(tile.get())
        .ok()
        .map(|tile| tile.children.iter().copied().collect())
        .unwrap_or_default()
}

fn format_entity_description(
    entity: Entity,
    items: &Query<ItemQuery>,
    npcs: &Query<NpcQuery>,
    players: &Query<PlayerQuery>,
) -> Option<String> {
    if let Ok(item) = items.get(entity) {
        return Some(format!(
            "#{}: {}",
            item.entity.index(),
            item.depiction.short_name
        ));
    }

    if let Ok(npc) = npcs.get(entity) {
        return Some(format!(
            "#{}: {}",
            npc.entity.index(),
            npc.depiction.short_name
        ));
    }

    if let Ok(player) = players.get(entity) {
        return Some(format!(
            "#{}: {}",
            player.entity.index(),
            player.character.name
        ));
    }

    None
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
        let target = handle_scan("scan rock");
        assert_eq!(target, Ok(Command::Scan((false, Some("rock".into())))));

        let inventory = handle_scan("scan inventory");
        assert_eq!(inventory, Ok(Command::Scan((true, None))));

        let no_target = handle_scan("scan");
        assert_eq!(no_target, Ok(Command::Scan((false, None))));
    }

    #[test]
    fn at_tile() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, scan);

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
        app.add_systems(Update, scan);

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
        app.add_systems(Update, scan);

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
