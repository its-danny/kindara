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
    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        utils::{get_message_content, send_message},
    };

    #[test]
    fn lists_online_characters() {
        let mut app = AppBuilder::new().build();
        app.add_system(who);

        let (client_id, _) = PlayerBuilder::new().name("Ashur").build(&mut app);
        PlayerBuilder::new().name("Bau").build(&mut app);

        send_message(&mut app, client_id, "who");

        app.update();

        let content = get_message_content(&mut app, client_id);

        assert_eq!(content, "Ashur, Bau");
    }
}
