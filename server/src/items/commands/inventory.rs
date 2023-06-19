use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    items::components::{Inventory, Item},
    player::components::{Client, Online},
    value_or_continue,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_inventory(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(inventory|inv|i)$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Inventory),
    }
}

pub fn inventory(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &Children), With<Online>>,
    inventories: Query<Option<&Children>, With<Inventory>>,
    items: Query<&Depiction, With<Item>>,
) {
    for command in commands.iter() {
        if let Command::Inventory = &command.command {
            let (client, children) =
                value_or_continue!(players.iter_mut().find(|(c, _)| c.id == command.from));
            let inventory = value_or_continue!(children
                .iter()
                .find_map(|child| inventories.get(*child).ok()));

            let mut items = inventory
                .iter()
                .flat_map(|children| children.iter())
                .filter_map(|child| items.get(*child).ok())
                .collect::<Vec<_>>();

            if items.is_empty() {
                outbox.send_text(client.id, "You are not carrying anything.");

                continue;
            }

            items.sort_by(|a, b| a.name.cmp(&b.name));

            let names = items
                .iter()
                .map(|item| item.name.clone())
                .collect::<Vec<_>>()
                .join(", ");

            outbox.send_text(client.id, format!("You are carrying: {names}"));
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
    fn carrying_nothing() {
        let mut app = AppBuilder::new().build();
        app.add_system(inventory);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        send_message(&mut app, client_id, "inventory");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You are not carrying anything.");
    }

    #[test]
    fn carrying_an_item() {
        let mut app = AppBuilder::new().build();
        app.add_system(inventory);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let item = ItemBuilder::new().name("stick").build(&mut app);

        let (_, client_id, inventory) = PlayerBuilder::new()
            .tile(tile)
            .has_inventory()
            .build(&mut app);

        app.world.entity_mut(item).set_parent(inventory.unwrap());

        send_message(&mut app, client_id, "inventory");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You are carrying: stick");
    }
}
