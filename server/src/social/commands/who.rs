use bevy::prelude::*;
use bevy_nest::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::{
    input::events::{Command, ParsedCommand},
    player::components::{Character, Client},
};

static REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^who$").unwrap());

pub fn parse_who(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if REGEX.is_match(content) {
        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Who,
        });

        true
    } else {
        false
    }
}

pub fn who(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character)>,
) {
    for command in commands.iter() {
        if let Command::Who = &command.command {
            let Some((client, _)) = players.iter().find(|(c, _)| c.id == command.from) else {
                return;
            };

            let online = players
                .iter()
                .map(|(_, character)| character.name.clone())
                .collect::<Vec<_>>();

            outbox.send_text(client.id, online.join(", "));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        spatial::components::Zone,
        test::{app_builder::AppBuilder, player_builder::PlayerBuilder, tile_builder::TileBuilder},
        world::resources::TileMap,
    };

    #[test]
    fn test_who() {
        let mut app = AppBuilder::new();
        app.add_system(who);

        let tile = TileBuilder::new().build(&mut app);

        app.world
            .resource_mut::<TileMap>()
            .insert((Zone::Void, IVec3::ZERO), tile);

        let (client_id, _) = PlayerBuilder::new().name("Morrigan").build(&mut app);
        PlayerBuilder::new().name("Astrid").build(&mut app);

        app.world.resource_mut::<Events<Inbox>>().send(Inbox {
            from: client_id,
            content: Message::Text("who".into()),
        });

        app.update();

        let outbox_events = app.world.resource::<Events<Outbox>>();
        let mut outbox_reader = outbox_events.get_reader();

        let sender_response = outbox_reader
            .iter(outbox_events)
            .find(|r| r.to == client_id)
            .expect("Expected response");

        assert_eq!(sender_response.to, client_id);

        let content = match &sender_response.content {
            Message::Text(text) => text,
            _ => panic!("Expected text message"),
        };

        assert_eq!(content, "Morrigan, Astrid");
    }
}
