use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*, utils::HashMap};
use bevy_mod_sysfail::sysfail;
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
        components::{Action, Door, Position, Tile, Transition, Zone},
        utils::offset_for_direction,
    },
    timed_paint,
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

#[derive(WorldQuery)]
pub struct ItemQuery {
    entity: Entity,
    depiction: &'static Depiction,
    surface: Option<&'static Surface>,
    door: Option<&'static Door>,
    children: Option<&'static Children>,
    with_item: With<Item>,
}

#[derive(WorldQuery)]
pub struct NpcQuery {
    entity: Entity,
    depiction: &'static Depiction,
    interactions: Option<&'static Interactions>,
    with_npc: With<Npc>,
}

#[derive(WorldQuery)]
pub struct PlayerQuery {
    entity: Entity,
    client: &'static Client,
    character: &'static Character,
    parent: &'static Parent,
    action: Option<&'static Action>,
    with_online: With<Online>,
}

#[derive(WorldQuery)]
pub struct TileQuery {
    entity: Entity,
    tile: &'static Tile,
    sprite: &'static Sprite,
    position: &'static Position,
    children: Option<&'static Children>,
    parent: &'static Parent,
}

#[sysfail(log)]
pub fn look(
    items: Query<ItemQuery>,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut prompts: EventWriter<Prompt>,
    npcs: Query<NpcQuery>,
    players: Query<PlayerQuery>,
    tiles: Query<TileQuery>,
    transitions: Query<(Entity, &Depiction), With<Transition>>,
    world_time: Res<WorldTime>,
    zones: Query<(&Zone, &Children)>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Look(target) = &command.command {
            let player = players
                .iter()
                .find(|p| p.client.id == command.from)
                .context("Player not found")?;

            let tile = tiles.get(player.parent.get())?;

            let output: String;

            if let Some(target) = target {
                output = look_at_item(target, &items, &tile.children)
                    .or_else(|| look_at_transition(target, &transitions, &tile.children))
                    .or_else(|| look_at_npc(target, &npcs, &tile.children))
                    .or_else(|| look_at_player(target, &players, &tile.children))
                    .unwrap_or_else(|| format!("You don't see a {} here.", target));
            } else {
                let (zone, zone_tiles) = zones.get(tile.parent.get())?;

                let exits = get_exits(tile.position, zone_tiles, &tiles);
                let items_line = get_items_line(tile.children, &items);
                let npcs_line = get_npcs_line(tile.children, &npcs);
                let players_line = get_players_line(player.client, tile.children, &players);

                output = if player.character.config.brief {
                    format!("{} {}{}", tile.sprite.character, tile.tile.name, exits)
                } else {
                    timed_paint!(
                        &world_time,
                        "{} {}{} - {} ({})\n{}{}{}{}",
                        tile.sprite.character,
                        tile.tile.name,
                        exits,
                        zone.name,
                        world_time.time_string(),
                        tile.tile.description,
                        items_line,
                        npcs_line,
                        players_line,
                    )
                };
            }

            outbox.send_text(player.client.id, output);

            prompts.send(Prompt::new(player.client.id));
        }
    }

    Ok(())
}

fn look_at_item(
    target: &str,
    items: &Query<ItemQuery>,
    siblings: &Option<&Children>,
) -> Option<String> {
    siblings
        .iter()
        .flat_map(|siblings| siblings.iter())
        .find_map(|&sibling| items.get(sibling).ok())
        .filter(|item| item.depiction.matches_query(&item.entity, target))
        .map(|item| {
            let surface_line = item
                .surface
                .and_then(|s| item.children.map(|c| (s, c)))
                .map(|(surface, children)| {
                    let on_surface = children
                        .iter()
                        .filter_map(|child| {
                            items.get(*child).ok().filter(|item| item.depiction.visible)
                        })
                        .map(|item| item.depiction.short_name.clone())
                        .collect::<Vec<String>>();

                    let on_surface = if on_surface.is_empty() {
                        "".to_string()
                    } else {
                        name_list(&on_surface, None, true)
                    };

                    if on_surface.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " {} the {} {} {}.",
                            to_title_case(&surface.kind.to_string()),
                            item.depiction.short_name,
                            if children.len() > 1 { "are" } else { "is" },
                            on_surface
                        )
                    }
                })
                .unwrap_or_default();

            let door_line = item
                .door
                .map(|door| {
                    if door.is_open {
                        " It is open."
                    } else {
                        " It is closed."
                    }
                })
                .unwrap_or_default();

            paint!(
                "{}{}{}",
                item.depiction.description,
                surface_line,
                door_line
            )
        })
}

fn look_at_transition(
    target: &str,
    transitions: &Query<(Entity, &Depiction), With<Transition>>,
    siblings: &Option<&Children>,
) -> Option<String> {
    siblings
        .iter()
        .flat_map(|siblings| siblings.iter())
        .find_map(|&sibling| transitions.get(sibling).ok())
        .filter(|(entity, depiction)| depiction.matches_query(entity, target))
        .map(|(_, depiction)| paint!("{}", depiction.description))
}

fn look_at_npc(
    target: &str,
    npcs: &Query<NpcQuery>,
    siblings: &Option<&Children>,
) -> Option<String> {
    siblings
        .iter()
        .flat_map(|siblings| siblings.iter())
        .find_map(|&sibling| npcs.get(sibling).ok())
        .filter(|npc| npc.depiction.matches_query(&npc.entity, target))
        .map(|npc| paint!("{}", npc.depiction.description))
}

fn look_at_player(
    target: &str,
    players: &Query<PlayerQuery>,
    siblings: &Option<&Children>,
) -> Option<String> {
    siblings
        .iter()
        .flat_map(|siblings| siblings.iter())
        .find_map(|&sibling| players.get(sibling).ok())
        .filter(|player| player.character.name.eq_ignore_ascii_case(target))
        .map(|player| {
            player.character.description.clone().unwrap_or_else(|| {
                format!(
                    "You can't quite make out what {} looks like.",
                    player.character.name
                )
            })
        })
}

fn get_exits(position: &Position, zone_tiles: &Children, tiles: &Query<TileQuery>) -> String {
    let directions = ["n", "ne", "e", "se", "s", "sw", "w", "nw", "u", "d"];

    let exits: Vec<String> = zone_tiles
        .iter()
        .filter_map(|&tile_entity| tiles.get(tile_entity).ok())
        .flat_map(|tile| {
            directions.iter().filter_map(move |&direction| {
                offset_for_direction(direction)
                    .filter(|&offset| tile.position.0 == position.0 + offset)
                    .map(|_| direction.to_uppercase())
            })
        })
        .collect();

    if exits.is_empty() {
        "".into()
    } else {
        format!(" [{}]", exits.join(", "))
    }
}

fn get_players_line(
    client: &Client,
    siblings: Option<&Children>,
    players: &Query<PlayerQuery>,
) -> String {
    if siblings.is_none() {
        return "".into();
    }

    let mut players_found: HashMap<String, Vec<String>> = HashMap::new();

    siblings
        .unwrap()
        .iter()
        .filter_map(|child| players.get(*child).ok())
        .filter(|player| player.client.id != client.id)
        .for_each(|player| {
            let action_phrase = player
                .action
                .map_or_else(|| "".to_string(), |a| a.0.clone());

            players_found
                .entry(action_phrase)
                .or_default()
                .push(player.character.name.clone());
        });

    if players_found.is_empty() {
        return "".into();
    }

    let players_found = players_found
        .into_iter()
        .map(|(phrase, names)| {
            let player_names = name_list(&names, Some(Color::Player), false);
            let verb = if names.len() > 1 { "are" } else { "is" };
            let action_description = if phrase.is_empty() {
                format!("{} standing here", verb)
            } else {
                phrase
            };

            format!("{} {}", player_names, action_description)
        })
        .collect::<Vec<String>>()
        .join(", ");

    format!("\n\n{players_found}.")
}

fn get_npcs_line(siblings: Option<&Children>, npcs: &Query<NpcQuery>) -> String {
    let npcs_found = siblings
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|sibling| npcs.get(*sibling).ok())
        .filter(|npc| npc.depiction.visible)
        .map(|npc| npc.depiction.short_name.clone())
        .collect::<Vec<String>>();

    if npcs_found.is_empty() {
        return "".into();
    }

    let npc_names = name_list(&npcs_found, Some(Color::Friendly), true);

    let formatted = format!(
        "{}{}",
        npc_names.chars().next().unwrap_or_default().to_uppercase(),
        &npc_names[1..]
    );

    format!("\n\n{} stand here.", formatted,)
}

fn get_items_line(siblings: Option<&Children>, items: &Query<ItemQuery>) -> String {
    let items_found = siblings
        .iter()
        .flat_map(|children| children.iter())
        .filter_map(|sibling| items.get(*sibling).ok())
        .filter(|item| item.depiction.visible)
        .map(|item| item.depiction.short_name.clone())
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
        let target = handle_look("look at rock");
        assert_eq!(target, Ok(Command::Look(Some("rock".into()))));

        let no_target = handle_look("look");
        assert_eq!(no_target, Ok(Command::Look(None)));

        let no_at = handle_look("look rock");
        assert_eq!(no_at, Ok(Command::Look(Some("rock".into()))));
    }

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
