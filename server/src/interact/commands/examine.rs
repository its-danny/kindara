use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand, ProxyCommand},
    interact::components::{InMenu, Interaction, Interactions, MenuType},
    items::{components::Item, utils::item_matches_query},
    player::components::{Client, Online},
    spatial::components::Tile,
    value_or_continue,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_examine(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| {
        Regex::new(r"^(examine|ex)(?: do (?P<option>\d+))?(?: (?P<target>.*))?$").unwrap()
    });

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let option = captures
                .name("option")
                .and_then(|m| m.as_str().parse::<usize>().ok());

            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            if option.is_none() && target.is_none() {
                return Err(ParseError::InvalidArguments("Examine what?".into()));
            }

            Ok(Command::Examine((target, option)))
        }
    }
}

pub fn examine(
    items: Query<(Entity, &Item, Option<&Interactions>)>,
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut proxy: EventWriter<ProxyCommand>,
    players: Query<(Entity, &Client, &Parent, Option<&InMenu>), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) {
    for command in commands.iter() {
        if let Command::Examine((target, option)) = &command.command {
            let (player, client, tile, in_menu) =
                value_or_continue!(players.iter().find(|(_, c, _, _)| c.id == command.from));
            let siblings = value_or_continue!(tiles.get(tile.get()).ok());

            if let Some(target) = target {
                let item = siblings
                    .iter()
                    .filter_map(|sibling| items.get(*sibling).ok())
                    .find(|(entity, item, _)| item_matches_query(entity, item, target));

                let Some((entity, item, interactions)) = item else {
                    outbox.send_text(client.id, format!("You don't see a {target} here."));

                    continue;
                };

                if let Some(interactions) = interactions {
                    let opts: Vec<String> = interactions
                        .0
                        .iter()
                        .enumerate()
                        .filter(|(_, i)| i.usable_in_menu())
                        .map(|(idx, int)| format!("[{}] {int}", idx + 1))
                        .collect();

                    outbox.send_text(
                        client.id,
                        format!(
                            "After thorough inspection, you find you are able to do the following:\n\n{}",
                            opts.join(", ")
                        ),
                    );

                    bevy.entity(player)
                        .insert(InMenu(MenuType::Examine(entity)));
                } else {
                    outbox.send_text(client.id, format!("{} has no interactions.", item.name));
                }
            }

            if let Some(option) = option {
                let Some(menu) = in_menu else {
                    outbox.send_text(client.id, "You are not in a menu.");

                    continue;
                };

                #[allow(irrefutable_let_patterns)]
                if let MenuType::Examine(entity) = menu.0 {
                    let (_, item, interactions) = value_or_continue!(items.get(entity).ok());

                    if let Some(interactions) = interactions {
                        let interaction = value_or_continue!(interactions
                            .0
                            .iter()
                            .filter(|i| i.usable_in_menu())
                            .nth(option - 1));

                        match interaction {
                            Interaction::Take => proxy.send(ProxyCommand(ParsedCommand {
                                from: client.id,
                                command: Command::Take((item.name.clone(), false, None)),
                            })),
                            _ => debug!("Unhandled interaction: {:?}", interaction),
                        }

                        bevy.entity(player).remove::<InMenu>();
                    } else {
                        outbox.send_text(client.id, format!("{} has no interactions.", item.name));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        items::commands::take::take,
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
    fn lists_interactions() {
        let mut app = AppBuilder::new().build();
        app.add_system(examine);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        ItemBuilder::new()
            .name("Rock")
            .short_name("rock")
            .interactions(vec![Interaction::Take])
            .tile(tile)
            .build(&mut app);

        let (_, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "examine rock");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(
            content,
            "After thorough inspection, you find you are able to do the following:\n\n[1] Take"
        );
    }

    #[test]
    fn performs_interaction() {
        let mut app = AppBuilder::new().build();
        app.add_systems((examine, take));

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let item = ItemBuilder::new()
            .name("Rock")
            .short_name("rock")
            .interactions(vec![Interaction::Take])
            .tile(tile)
            .build(&mut app);

        let (player, client_id, inventory) = PlayerBuilder::new()
            .has_inventory()
            .tile(tile)
            .build(&mut app);

        send_message(&mut app, client_id, "examine rock");
        app.update();

        assert!(app.world.get::<InMenu>(player).is_some());

        send_message(&mut app, client_id, "examine do 1");
        app.update();

        // Second update to process the ProxyCommand event
        app.update();

        assert!(app
            .world
            .get::<Children>(inventory.unwrap())
            .unwrap()
            .contains(&item));
    }
}
