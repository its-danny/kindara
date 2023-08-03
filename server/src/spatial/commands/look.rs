use std::sync::OnceLock;

use bevy::{prelude::*, utils::HashMap};
use bevy_nest::prelude::*;
use inflector::cases::titlecase::to_title_case;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::Interactions,
    items::components::{Item, Surface},
    npc::components::Npc,
    paint,
    player::{
        components::{Character, Client, Online},
        events::Prompt,
    },
    spatial::{
        components::{Action, Position, Tile, Transition, Zone},
        utils::offset_for_direction,
    },
    value_or_continue,
    visual::{
        components::{Depiction, Sprite},
        paint::Color,
        utils::name_list,
    },
    world::resources::WorldTime,
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
    items: Query<(Entity, &Depiction, Option<&Surface>, Option<&Children>), With<Item>>,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut prompts: EventWriter<Prompt>,
    npcs: Query<(Entity, &Depiction, Option<&Interactions>), With<Npc>>,
    players: Query<(&Client, &Character, &Parent, Option<&Action>), With<Online>>,
    tiles: Query<(&Tile, &Sprite, &Position, Option<&Children>, &Parent)>,
    transitions: Query<(Entity, &Depiction), With<Transition>>,
    world_time: Res<WorldTime>,
    zones: Query<(&Zone, &Children)>,
) {
    for command in commands.iter() {
        if let Command::Look(target) = &command.command {
            let (client, character, tile, _) =
                value_or_continue!(players.iter().find(|(c, _, _, _)| c.id == command.from));
            let (tile, sprite, position, siblings, zone) =
                value_or_continue!(tiles.get(tile.get()).ok());

            let output: String;

            if let Some(target) = target {
                let matching_item = siblings
                    .iter()
                    .flat_map(|siblings| siblings.iter())
                    .filter_map(|sibling| items.get(*sibling).ok())
                    .find(|(entity, depiction, _, _)| depiction.matches_query(entity, target));

                let matching_transition = siblings
                    .iter()
                    .flat_map(|siblings| siblings.iter())
                    .filter_map(|sibling| transitions.get(*sibling).ok())
                    .find(|(entity, depiction)| depiction.matches_query(entity, target));

                let matching_npc = siblings
                    .iter()
                    .flat_map(|siblings| siblings.iter())
                    .filter_map(|sibling| npcs.get(*sibling).ok())
                    .find(|(entity, depiction, _)| depiction.matches_query(entity, target));

                let matching_player = siblings
                    .iter()
                    .flat_map(|siblings| siblings.iter())
                    .filter_map(|sibling| players.get(*sibling).ok())
                    .find(|(_, c, _, _)| &c.name.to_lowercase() == target);

                if let Some((_, depiction, surface, children)) = matching_item {
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
                                    depiction.short_name,
                                    if children.len() > 1 { "are" } else { "is" },
                                    on_surface
                                )
                            }
                        })
                        .unwrap_or("".into());

                    output = paint!("{}{}", depiction.description, surface_line);
                } else if let Some((_, depiction)) = matching_transition {
                    output = paint!("{}", depiction.description,);
                } else if let Some((_, depiction, _)) = matching_npc {
                    output = paint!("{}", depiction.description,);
                } else if let Some((_, character, _, _)) = matching_player {
                    output = character.description.clone().unwrap_or(format!(
                        "You can't quite make out what {} looks like.",
                        character.name
                    ));
                } else {
                    output = format!("You don't see a {target} here.");
                }
            } else {
                let (zone, zone_tiles) = value_or_continue!(zones.get(zone.get()).ok());

                let exits = get_exits(position, zone_tiles, &tiles);
                let items_line = get_items_line(siblings, &items);
                let npcs_line = get_npcs_line(siblings, &npcs);
                let players_line = get_players_line(client, siblings, &players);

                output = if character.config.brief {
                    format!("{} {}{}", sprite.character, tile.name, exits)
                } else {
                    paint!(
                        "{} {}{} - {} ({})\n{}{}{}{}",
                        sprite.character,
                        tile.name,
                        exits,
                        zone.name,
                        world_time.time_string(),
                        tile.description,
                        items_line,
                        npcs_line,
                        players_line,
                    )
                };
            }

            outbox.send_text(client.id, output);

            prompts.send(Prompt::new(client.id));
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
    players: &Query<(&Client, &Character, &Parent, Option<&Action>), With<Online>>,
) -> String {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    let players_found = siblings
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|child| players.get(*child).ok())
        .filter(|(c, _, _, _)| c.id != client.id)
        .map(|(_, character, _, seated)| (character.name.clone(), seated))
        .collect::<Vec<(String, Option<&Action>)>>();

    for (name, seated) in &players_found {
        let entry = map
            .entry(seated.map_or("".into(), |s| s.0.clone()))
            .or_insert(vec![]);

        entry.push(name.clone())
    }

    if players_found.is_empty() {
        return "".into();
    }

    let players_found = map
        .iter()
        .map(|(phrase, names)| {
            let player_names = name_list(names, Some(Color::Player), false);

            format!(
                "{} {}",
                player_names,
                if phrase.is_empty() {
                    if names.len() > 1 {
                        "are standing here"
                    } else {
                        "is standing here"
                    }
                } else {
                    phrase
                }
            )
        })
        .collect::<Vec<String>>()
        .join(", ");

    format!("\n\n{players_found}.")
}

fn get_npcs_line(
    siblings: Option<&Children>,
    npcs: &Query<(Entity, &Depiction, Option<&Interactions>), With<Npc>>,
) -> String {
    let npcs_found = siblings
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|sibling| npcs.get(*sibling).ok())
        .filter(|(_, depiction, _)| depiction.visible)
        .map(|(_, depiction, _)| depiction.short_name.clone())
        .collect::<Vec<String>>();

    if npcs_found.is_empty() {
        return "".into();
    }

    let npc_names = name_list(&npcs_found, Some(Color::Npc), true);

    let formatted = format!(
        "{}{}",
        npc_names.chars().next().unwrap_or_default().to_uppercase(),
        &npc_names[1..]
    );

    format!("\n\n{} stand here.", formatted,)
}

fn get_items_line(
    siblings: Option<&Children>,
    items: &Query<(Entity, &Depiction, Option<&Surface>, Option<&Children>), With<Item>>,
) -> String {
    let items_found = siblings
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|sibling| items.get(*sibling).ok())
        .filter(|(_, depiction, _, _)| depiction.visible)
        .map(|(_, depiction, _, _)| depiction.short_name.clone())
        .collect::<Vec<String>>();

    if items_found.is_empty() {
        return "".into();
    }

    let item_names = name_list(&items_found, Some(Color::Item), true);

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
    items: &Query<(Entity, &Depiction, Option<&Surface>, Option<&Children>), With<Item>>,
    children: &Children,
) -> String {
    let on_surface = children
        .iter()
        .filter_map(|child| items.get(*child).ok())
        .filter(|(_, depiction, _, _)| depiction.visible)
        .map(|(_, depiction, _, _)| depiction.short_name.clone())
        .collect::<Vec<String>>();

    if on_surface.is_empty() {
        return "".into();
    }

    name_list(&on_surface, None, true)
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
    fn sends_tile_info() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

        let zone = ZoneBuilder::new().name("V").build(&mut app);
        let tile = TileBuilder::new()
            .sprite("x")
            .name("The Void")
            .description("A vast, empty void.")
            .build(&mut app, zone);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "x The Void - V (00:00am)\nA vast, empty void.");
    }

    #[test]
    fn sends_item_info() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

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

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "A small rock.");
    }

    #[test]
    fn sends_player_info() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        PlayerBuilder::new()
            .name("Ramos")
            .description("A big, burly hunk.")
            .tile(tile)
            .build(&mut app);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "look ramos");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "A big, burly hunk.");
    }

    #[test]
    fn items_on_surface() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let table = ItemBuilder::new()
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

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            "A small dining table. On the table is a dinner plate."
        );
    }

    #[test]
    fn has_exits() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

        let zone = ZoneBuilder::new().name("V").build(&mut app);

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

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            "x The Void [N, U] - V (00:00am)\nA vast, empty void."
        );
    }

    #[test]
    fn other_player() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

        let zone = ZoneBuilder::new().name("V").build(&mut app);
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

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            "x The Void - V (00:00am)\nA vast, empty void.\n\nAstrid is standing here."
        );
    }

    #[test]
    fn multiple_other_player() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

        let zone = ZoneBuilder::new().name("V").build(&mut app);
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

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            "x The Void - V (00:00am)\nA vast, empty void.\n\nAstrid and Ramos are standing here."
        );
    }

    #[test]
    fn one_item() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

        let zone = ZoneBuilder::new().name("V").build(&mut app);
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

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            "x The Void - V (00:00am)\nA vast, empty void.\n\nA rock lies on the ground."
        );
    }

    #[test]
    fn multiple_of_the_same_item() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

        let zone = ZoneBuilder::new().name("V").build(&mut app);
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

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            "x The Void - V (00:00am)\nA vast, empty void.\n\n2 rocks lie on the ground."
        );
    }

    #[test]
    fn multiple_of_different_items() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, look);

        let zone = ZoneBuilder::new().name("V").build(&mut app);
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

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            "x The Void - V (00:00am)\nA vast, empty void.\n\nA rock and a stick lie on the ground."
        );
    }
}
