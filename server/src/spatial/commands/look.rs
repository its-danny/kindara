use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use inflector::cases::titlecase::to_title_case;
use regex::Regex;
use vari::vformat;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    items::{
        components::{Item, Surface},
        utils::{item_matches_query, item_name_list},
    },
    player::components::{Character, Client, Online},
    spatial::{
        components::{Position, Tile, Zone},
        utils::offset_for_direction,
    },
    value_or_continue,
    visual::components::Sprite,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_look(content: &str) -> Result<Command, ParseError> {
    let regex =
        REGEX.get_or_init(|| Regex::new(r"^(look|l)( (at|in))?( (?P<target>.*))?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Look(target))
        }
    }
}

pub fn look(
    items: Query<(Entity, &Item, Option<&Surface>, Option<&Children>)>,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Parent), With<Online>>,
    tiles: Query<(&Tile, &Sprite, &Position, Option<&Children>, &Parent)>,
    zones: Query<(&Zone, &Children)>,
) {
    for command in commands.iter() {
        if let Command::Look(target) = &command.command {
            let (client, character, tile) =
                value_or_continue!(players.iter().find(|(c, _, _)| c.id == command.from));
            let (tile, sprite, position, siblings, zone) =
                value_or_continue!(tiles.get(tile.get()).ok());

            let output: String;

            if let Some(target) = target {
                let item = siblings
                    .iter()
                    .flat_map(|siblings| siblings.iter())
                    .filter_map(|sibling| items.get(*sibling).ok())
                    .find(|(entity, item, _, _)| item_matches_query(entity, item, target));

                if let Some((_, item, surface, children)) = item {
                    let surface_line = surface
                        .and_then(|s| children.map(|c| (s, c)))
                        .map(|(surface, children)| {
                            let on_surface = items_on_surface(&items, children);

                            if on_surface.is_empty() {
                                "".into()
                            } else {
                                format!(
                                    " {} the {} {} {}.",
                                    to_title_case(&surface.kind.to_string()),
                                    item.short_name,
                                    if children.len() > 1 { "are" } else { "is" },
                                    on_surface
                                )
                            }
                        })
                        .unwrap_or("".into());

                    output = vformat!("{}\n{}{}", item.name, item.description, surface_line);
                } else {
                    output = format!("You don't see a {target} here.");
                }
            } else {
                let (_, zone_tiles) = value_or_continue!(zones.get(zone.get()).ok());

                let exits = get_exits(position, zone_tiles, &tiles);
                let items_line = get_items_line(siblings, &items);
                let players_line = get_players_line(client, siblings, &players);

                output = if character.config.brief {
                    format!("{} {}{}", sprite.character, tile.name, exits)
                } else {
                    vformat!(
                        "{} {}{}\n{}{}{}",
                        sprite.character,
                        tile.name,
                        exits,
                        tile.description,
                        items_line,
                        players_line,
                    )
                };
            }

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
        1 => vformat!("[$cyan]{}[$/]", player_names[0]),
        2 => vformat!(
            "[$cyan]{}[$/] and [$cyan]{}[$/]",
            player_names[0],
            player_names[1]
        ),
        _ => {
            let last = player_names.pop().unwrap_or_default();

            vformat!(
                "[$cyan]{}[$/], and [$cyan]{}[$/]",
                player_names.join(", "),
                last
            )
        }
    };

    format!(
        "\n\n{} {} here.",
        player_names_concat,
        if player_names.len() == 1 { "is" } else { "are" }
    )
}

fn get_items_line(
    siblings: Option<&Children>,
    items: &Query<(Entity, &Item, Option<&Surface>, Option<&Children>)>,
) -> String {
    let items_found = siblings
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|sibling| items.get(*sibling).ok())
        .filter(|(_, item, _, _)| item.visible)
        .map(|(_, item, _, _)| item.short_name.clone())
        .collect::<Vec<String>>();

    if items_found.is_empty() {
        return "".into();
    }

    let item_names = item_name_list(&items_found);

    let formatted = format!(
        "{}{}",
        item_names.chars().next().unwrap_or_default().to_uppercase(),
        &item_names[1..]
    );

    format!(
        "\n\n{} {} on the ground.",
        formatted,
        if items_found.len() == 1 {
            "lies"
        } else {
            "lie"
        }
    )
}

fn items_on_surface(
    items: &Query<(Entity, &Item, Option<&Surface>, Option<&Children>)>,
    children: &Children,
) -> String {
    let on_surface = children
        .iter()
        .filter_map(|child| items.get(*child).ok())
        .filter(|(_, item, _, _)| item.visible)
        .map(|(_, item, _, _)| item.short_name.clone())
        .collect::<Vec<String>>();

    if on_surface.is_empty() {
        return "".into();
    }

    item_name_list(&on_surface)
}

#[cfg(test)]
mod tests {
    use vari::util::NoAnsi;

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
    fn sends_item_info() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        ItemBuilder::new()
            .name("Rock")
            .description("A small rock.")
            .tile(tile)
            .build(&mut app);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look rock");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Rock\nA small rock.");
    }

    #[test]
    fn items_on_surface() {
        let mut app = AppBuilder::new().build();
        app.add_system(look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let table = ItemBuilder::new()
            .name("Dining Table")
            .short_name("table")
            .description("A small dining table.")
            .is_surface(SurfaceKind::Floor, 1)
            .tile(tile)
            .build(&mut app);

        let plate = ItemBuilder::new()
            .short_name("dinner plate")
            .build(&mut app);
        app.world.entity_mut(table).add_child(plate);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look table");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(
            content,
            "Dining Table\nA small dining table. On the table is a dinner plate."
        );
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
            content.no_ansi(),
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
            content.no_ansi(),
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
            .short_name("rock")
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
            .short_name("rock")
            .tile(tile)
            .build(&mut app);
        ItemBuilder::new()
            .short_name("rock")
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
            .short_name("rock")
            .tile(tile)
            .build(&mut app);
        ItemBuilder::new()
            .short_name("stick")
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
