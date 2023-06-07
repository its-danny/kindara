use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Client},
    spatial::{
        components::{Position, Tile, Transition},
        utils::view_for_tile,
    },
    visual::components::Sprite,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_enter(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(enter)(?P<transition> .+)?$").unwrap());

    if let Some(captures) = regex.captures(content) {
        let target = captures.name("transition").map(|m| m.as_str().trim().to_string());

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Enter(target),
        });

        true
    } else {
        false
    }
}

pub fn enter(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(Entity, &Client, &Parent)>,
    tiles: Query<(Entity, &Position, &Tile, &Sprite, Option<&Children>)>,
    transitions: Query<&Transition>,
) {
    for command in commands.iter() {
        if let Command::Enter(target) = &command.command {
            let Some((player, client, tile)) = players.iter_mut().find(|(_, c, _)| c.id == command.from) else {
                debug!("Could not find player for client: {:?}", command.from);

                continue;
            };

            let Ok((_, _, _, _, siblings)) = tiles.get(tile.get()) else {
                debug!("Could not get parent: {:?}", tile.get());

                continue;
            };

            let transitions = siblings.map(|siblings| {
                siblings.iter().filter_map(|child| transitions.get(*child).ok()).collect::<Vec<_>>()
            }).unwrap_or_else(|| vec![]);

            if transitions.is_empty() {
                outbox.send_text(client.id, "There is nowhere to enter from here.");
                
                continue;
            }

            let Some(transition) = transitions.iter().find(|transition| {
                target
                    .as_ref()
                    .map_or(true, |tag| transition.tags.contains(tag))
            }) else {
                outbox.send_text(client.id, "Could not find entrance.");

                continue;
            };

            let Some((target, _, tile, sprite, _)) = tiles.iter().find(|(_, p, _, _, _)| {
                p.zone == transition.zone && p.coords == transition.coords
            }) else {
                debug!("Could not find tile for transition: {:?}", transition);
                
                continue;
            };

            bevy.entity(player).set_parent(target);

            outbox.send_text(client.id, view_for_tile(tile, sprite, false));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        spatial::components::Zone,
        test::{
            app_builder::AppBuilder,
            player_builder::PlayerBuilder,
            tile_builder::TileBuilder,
            transition_builder::TransitionBuilder,
            utils::{get_message_content, send_message},
        },
    };

    use super::*;

    #[test]
    fn enters_by_tag() {
        let mut app = AppBuilder::new().build();
        app.add_system(enter);

        let start = TileBuilder::new().zone(Zone::Void).build(&mut app);
        let destination = TileBuilder::new().zone(Zone::Movement).build(&mut app);

        TransitionBuilder::new(start, destination)
            .tags(&vec!["movement"])
            .build(&mut app);

        let (client_id, player) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "enter movement");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), destination);
    }

    #[test]
    fn enters_first_if_no_tag() {
        let mut app = AppBuilder::new().build();
        app.add_system(enter);

        let start = TileBuilder::new().zone(Zone::Void).build(&mut app);
        let first = TileBuilder::new().zone(Zone::Movement).build(&mut app);
        let second = TileBuilder::new().zone(Zone::Movement).coords(IVec3::new(1, 1, 1)).build(&mut app);

        TransitionBuilder::new(start, first).build(&mut app);
        TransitionBuilder::new(start, second).build(&mut app);

        let (client_id, player) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "enter");
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), first);
    }

    #[test]
    fn transition_not_found() {
        let mut app = AppBuilder::new().build();
        app.add_system(enter);

        let start = TileBuilder::new().zone(Zone::Void).build(&mut app);
        let destination = TileBuilder::new().zone(Zone::Movement).build(&mut app);

        TransitionBuilder::new(start, destination)
            .tags(&vec!["movement"])
            .build(&mut app);

        let (client_id, _) = PlayerBuilder::new().tile(start).build(&mut app);

        send_message(&mut app, client_id, "enter at your own risk");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert!(content.contains("Could not find entrance."));
    }

    #[test]
    fn no_transition() {
        let mut app = AppBuilder::new().build();
        app.add_system(enter);

        let tile = TileBuilder::new().build(&mut app);

        let (client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "enter the dragon");
        app.update();

        let content = get_message_content(&mut app, client_id);

        assert!(content.contains("There is nowhere to enter from here."));
    }
}
