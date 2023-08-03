use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::Interactions,
    items::components::Seat,
    paint,
    player::components::{Client, Online},
    spatial::components::{Action, Tile},
    value_or_continue,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_sit(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^sit( (?P<target>.+))?").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Sit(target))
        }
    }
}

pub fn sit(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(Entity, &Client, &Parent), With<Online>>,
    tiles: Query<Option<&Children>, With<Tile>>,
    seats: Query<(Entity, &Interactions, &Seat, &Depiction)>,
) {
    for command in commands.iter() {
        if let Command::Sit(target) = &command.command {
            let (player, client, tile) =
                value_or_continue!(players.iter_mut().find(|(_, c, _)| c.id == command.from));

            let siblings = value_or_continue!(tiles.get(tile.get()).ok());

            let Some(target) = target else {
                bevy.entity(player).insert(Action("on the floor".into()));

                outbox.send_text(client.id, "You sit on the floor.");

                continue;
            };

            let Some((_, _, seat, _)) = siblings
                .iter()
                .flat_map(|siblings| siblings.iter())
                .filter_map(|sibling| seats.get(*sibling).ok())
                .find(|(entity, _, _, depiction)| depiction.matches_query(entity, target)) else {
                    outbox.send_text(client.id, "You can't sit there.");

                    continue;
            };

            bevy.entity(player).insert(Action(seat.phrase.clone()));

            outbox.send_text(client.id, paint!("You sit {}.", seat.phrase));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        interact::components::Interaction,
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
    fn sit_on_the_floor() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, sit);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "sit");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You sit on the floor.");

        assert!(app.world.entity(player).get::<Action>().is_some());
    }

    #[test]
    fn sit_on_chair() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, sit);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);
        let chair = ItemBuilder::new()
            .name("chair")
            .tile(tile)
            .interactions(vec![Interaction::Sit])
            .build(&mut app);

        app.world.entity_mut(chair).insert(Seat {
            phrase: "on the chair".into(),
        });

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "sit chair");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You sit on the chair.");

        assert_eq!(
            app.world.entity(player).get::<Action>().unwrap().0,
            "on the chair"
        );
    }
}
