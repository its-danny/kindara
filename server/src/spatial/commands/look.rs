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
    spatial::{
        components::{Position, Tile, Zone},
        utils::offset_for_direction,
    },
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
    items: Query<&Item>,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent), With<Online>>,
    tiles: Query<(&Tile, &Sprite, &Position, Option<&Children>, &Parent)>,
    zones: Query<(&Zone, &Children)>,
) {
    for command in commands.iter() {
        if let Command::Look = &command.command {
            let Some((client, character, tile)) = players.iter().find(|(c, _, _)| c.id == command.from) else {
                debug!("Could not find authenticated client: {:?}", command.from);

                continue;
            };

            let Ok((tile, sprite, position, siblings, zone)) = tiles.get(tile.get()) else {
                debug!("Could not get tile: {:?}", tile.get());

                continue;
            };

            let Ok((_, zone_tiles)) = zones.get(zone.get()) else {
                debug!("Could not get zone: {:?}", zone.get());

                continue;
            };

            let exits = get_exits(position, zone_tiles, &tiles);
            let items_line = get_items_line(siblings, &items);
            let players_line = get_players_line(client, siblings, &players);

            let output = if character.config.brief {
                format!("{} {}{}", sprite.character, tile.name, exits)
            } else {
                format!(
                    "{} {}{}\n{}{}{}",
                    sprite.character, tile.name, exits, tile.description, items_line, players_line,
                )
            };

            outbox.send_text(client.id, output);
        }
    }
}

fn get_exits(
    position: &Position,
    zone_tiles: &Children,
    tiles: &Query<(&Tile, &Sprite, &Position, Option<&Children>, &Parent)>,
) -> String {
    let directions = vec!["n", "ne", "e", "se", "s", "sw", "w", "nw", "u", "d"];
    let mut exits: Vec<String> = vec![];

    for tile in zone_tiles.iter() {
        if let Ok((_, _, p, _, _)) = tiles.get(*tile) {
            directions.iter().for_each(|direction| {
                if p.0 == position.0 + offset_for_direction(direction).unwrap() {
                    exits.push(direction.to_uppercase());
                }
            });
        }
    }

    if exits.is_empty() {
        return "".into();
    }

    format!(" [{}]", exits.join(", "))
}

fn get_players_line(
    client: &Client,
    siblings: Option<&Children>,
    players: &Query<(&Client, &Character, &Parent), With<Online>>,
) -> String {
    let mut player_names = siblings
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| players.get(*child).ok())
        .filter(|(c, _, _)| c.id != client.id)
        .map(|(_, character, _)| character.name.clone())
        .collect::<Vec<String>>();

    if player_names.is_empty() {
        return "".into();
    }

    player_names.sort();

    let player_names_concat = match player_names.len() {
        1 => player_names[0].clone(),
        2 => format!("{} and {}", player_names[0], player_names[1]),
        _ => {
            let last = player_names.pop().unwrap_or_default();

            format!("{}, and {}", player_names.join(", "), last)
        }
    };

    format!(
        "\n\n{} {} here.",
        player_names_concat,
        if player_names.len() == 1 { "is" } else { "are" }
    )
}

fn get_items_line(siblings: Option<&Children>, items: &Query<&Item>) -> String {
    let counted_item_names: HashMap<String, u16> = siblings
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| items.get(*child).ok())
        .map(|item| item.name_on_ground.clone())
        .fold(HashMap::new(), |mut map, name| {
            *map.entry(name).or_default() += 1;

            map
        });

    if counted_item_names.is_empty() {
        return "".into();
    }

    let mut item_names = counted_item_names
        .iter()
        .map(|(name, count)| {
            if *count > 1 {
                format!("{} {}", count, to_plural(name))
            } else {
                indefinite(name)
            }
        })
        .collect::<Vec<_>>();

    item_names.sort();

    let item_names_concat = match item_names.len() {
        1 => item_names[0].clone(),
        2 => format!("{} and {}", item_names[0], item_names[1]),
        _ => {
            let last = item_names.pop().unwrap_or_default();

            format!("{}, and {}", item_names.join(", "), last)
        }
    };

    let item_names_formatted = format!(
        "{}{}",
        item_names_concat
            .chars()
            .next()
            .unwrap_or_default()
            .to_uppercase(),
        &item_names_concat[1..]
    );

    format!(
        "\n\n{} {} on the ground.",
        item_names_formatted,
        if counted_item_names.values().sum::<u16>() == 1 {
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

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "x The Void\nA vast, empty void.");
    }

    #[test]
    fn has_exits() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let zone = ZoneBuilder::new().build(&mut app);

        let tile = TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .position(IVec3::new(0, 0, 0))
            .build(&mut app, zone);
        TileBuilder::new()
            .position(IVec3::new(0, -1, 0))
            .build(&mut app, zone);
        TileBuilder::new()
            .position(IVec3::new(0, 0, 1))
            .build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "x The Void [N, U]\nA vast, empty void.");
    }

    #[test]
    fn other_player() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        PlayerBuilder::new()
            .tile(tile)
            .name("Astrid")
            .build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(
            content,
            "x The Void\nA vast, empty void.\n\nAstrid is here."
        );
    }

    #[test]
    fn multiple_other_player() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        PlayerBuilder::new()
            .tile(tile)
            .name("Astrid")
            .build(&mut app);

        PlayerBuilder::new()
            .tile(tile)
            .name("Ramos")
            .build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(
            content,
            "x The Void\nA vast, empty void.\n\nAstrid and Ramos are here."
        );
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

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

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

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

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

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(
            content,
            "x The Void\nA vast, empty void.\n\nA rock and a stick lie on the ground."
        );
    }
}
