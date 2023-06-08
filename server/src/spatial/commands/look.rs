use std::sync::OnceLock;

use bevy::{prelude::*, utils::HashMap};
use bevy_nest::prelude::*;
use indefinite::indefinite;
use inflector::string::pluralize::to_plural;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    items::components::Item,
    player::components::{Character, Client, Online},
    spatial::components::Tile,
    visual::components::Sprite,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_look(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(look|l)$").unwrap());

    if regex.is_match(content) {
        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Look,
        });

        true
    } else {
        false
    }
}

pub fn look(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent), With<Online>>,
    tiles: Query<(&Tile, &Sprite, &Children)>,
    items: Query<&Item>,
) {
    for command in commands.iter() {
        if let Command::Look = &command.command {
            let Some((client, character, tile)) = players.iter().find(|(c, _, _)| c.id == command.from) else {
                debug!("Could not find player for client: {:?}", command.from);

                continue;
            };

            let Ok((tile, sprite, children)) = tiles.get(tile.get()) else {
                debug!("Could not get tile: {:?}", tile.get());

                continue;
            };

            let items_line = generate_items_line(children, &items);

            let output = if character.config.brief {
                format!("{} {}", sprite.character, tile.name)
            } else {
                format!(
                    "{} {}\n{}{}",
                    sprite.character, tile.name, tile.description, items_line,
                )
            };

            outbox.send_text(client.id, output);
        }
    }
}

fn generate_items_line(children: &[Entity], items: &Query<&Item>) -> String {
    let item_names: HashMap<String, u16> = children
        .iter()
        .filter_map(|child| items.get(*child).ok())
        .map(|item| item.name_on_ground.clone())
        .fold(HashMap::new(), |mut map, name| {
            *map.entry(name).or_default() += 1;

            map
        });

    if item_names.is_empty() {
        return "".into();
    }

    let mut item_names_list = item_names
        .iter()
        .map(|(name, count)| {
            if *count > 1 {
                format!("{} {}", count, to_plural(name))
            } else {
                indefinite(name)
            }
        })
        .collect::<Vec<_>>();

    item_names_list.sort();

    let item_names_sentence = if item_names_list.len() > 1 {
        let last = item_names_list.pop().unwrap_or_default();

        format!("{}, and {}", item_names_list.join(", "), last)
    } else {
        item_names_list.join(", ")
    };

    let item_names_formatted = format!(
        "{}{}",
        item_names_sentence
            .chars()
            .next()
            .unwrap_or_default()
            .to_uppercase(),
        &item_names_sentence[1..]
    );

    format!(
        "\n\n{} {} on the ground.",
        item_names_formatted,
        if item_names.values().sum::<u16>() == 1 {
            "lies"
        } else {
            "lie"
        }
    )
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
    fn sends_tile_info() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .build(&mut app, zone);

        let (client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "x The Void\nA vast, empty void.");
    }

    #[test]
    fn one_item() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .build(&mut app, zone);

        ItemBuilder::new()
            .name_on_ground("rock")
            .tile(tile)
            .build(&mut app);

        let (client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(
            content,
            "x The Void\nA vast, empty void.\n\nA rock lies on the ground."
        );
    }

    #[test]
    fn multiple_of_the_same_item() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .build(&mut app, zone);

        ItemBuilder::new()
            .name_on_ground("rock")
            .tile(tile)
            .build(&mut app);
        ItemBuilder::new()
            .name_on_ground("rock")
            .tile(tile)
            .build(&mut app);

        let (client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(
            content,
            "x The Void\nA vast, empty void.\n\n2 rocks lie on the ground."
        );
    }

    #[test]
    fn multiple_of_different_items() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .build(&mut app, zone);

        ItemBuilder::new()
            .name_on_ground("rock")
            .tile(tile)
            .build(&mut app);
        ItemBuilder::new()
            .name_on_ground("stick")
            .tile(tile)
            .build(&mut app);

        let (client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(
            content,
            "x The Void\nA vast, empty void.\n\nA rock, and a stick lie on the ground."
        );
    }
}
