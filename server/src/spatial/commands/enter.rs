use bevy::prelude::*;
use bevy_nest::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
    spatial::{
        components::{Position, Tile, Transition},
        utils::view_for_tile,
    },
    visual::components::Sprite,
};
static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^(enter)(?P<transition> .+)?$").unwrap());

pub fn parse_enter(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if let Some(captures) = REGEX.captures(content) {
        let target = captures.name("transition").map(|m| m.as_str().to_string());

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
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Position), With<Character>>,
    transitions: Query<(&Position, &Transition), Without<Client>>,
    tiles: Query<(&Position, &Tile, &Sprite), Without<Client>>,
) {
    for command in commands.iter() {
        if let Command::Enter(target) = &command.command {
            let Some((client, mut player_position)) = players.iter_mut().find(|(c, _)| c.id == command.from) else {
                return;
            };

            let transition = transitions.iter().find(|(p, t)| {
                p.zone == player_position.zone
                    && p.coords == player_position.coords
                    && target
                        .as_ref()
                        .map_or(true, |tag| t.tags.contains(&tag.trim().to_string()))
            });

            if let Some((_, transition)) = transition {
                player_position.zone = transition.zone;
                player_position.coords = transition.coords;

                if let Some((_, tile, sprite)) = tiles.iter().find(|(p, _, _)| {
                    p.zone == player_position.zone && p.coords == player_position.coords
                }) {
                    outbox.send_text(client.id, view_for_tile(tile, sprite, false))
                }
            } else {
                outbox.send_text(client.id, "Enter what?");
            }
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
        let mut app = AppBuilder::new();
        app.add_system(enter);

        TileBuilder::new().zone(Zone::Void).build(&mut app);
        TileBuilder::new().zone(Zone::Movement).build(&mut app);

        TransitionBuilder::new()
            .tags(&vec!["movement"])
            .target_zone(Zone::Movement)
            .target_coords(IVec3::ZERO)
            .build(&mut app);

        let (client_id, player) = PlayerBuilder::new().zone(Zone::Void).build(&mut app);

        send_message(&mut app, client_id, "enter movement");

        app.update();

        let updated_position = app.world.get::<Position>(player).unwrap();

        assert_eq!(updated_position.zone, Zone::Movement);
        assert_eq!(updated_position.coords, IVec3::ZERO);
    }

    #[test]
    fn enters_first_if_no_tag() {
        let mut app = AppBuilder::new();
        app.add_system(enter);

        TileBuilder::new().zone(Zone::Void).build(&mut app);
        TileBuilder::new().zone(Zone::Movement).build(&mut app);

        TransitionBuilder::new()
            .target_zone(Zone::Movement)
            .target_coords(IVec3::ZERO)
            .build(&mut app);

        TransitionBuilder::new()
            .target_zone(Zone::Movement)
            .target_coords(IVec3::new(1, 1, 1))
            .build(&mut app);

        let (client_id, player) = PlayerBuilder::new().zone(Zone::Void).build(&mut app);

        send_message(&mut app, client_id, "enter");

        app.update();

        let updated_position = app.world.get::<Position>(player).unwrap();

        assert_eq!(updated_position.zone, Zone::Movement);
        assert_eq!(updated_position.coords, IVec3::ZERO);
    }

    #[test]
    fn no_transition() {
        let mut app = AppBuilder::new();
        app.add_system(enter);

        TileBuilder::new().build(&mut app);

        let (client_id, _) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "enter the dragon");

        app.update();

        let content = get_message_content(&mut app, client_id);

        assert!(content.contains("Enter what?"));
    }
}
