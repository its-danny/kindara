use std::sync::OnceLock;

use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Character, Client, Online},
    value_or_continue,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_who(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^who$").unwrap());

    match regex.is_match(content) {
        false => Err(ParseError::WrongCommand),
        true => Ok(Command::Who),
    }
}

pub fn who(
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character), With<Online>>,
) {
    for command in commands.iter() {
        if let Command::Who = &command.command {
            let (client, _) =
                value_or_continue!(players.iter().find(|(c, _)| c.id == command.from));

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
        app.add_systems(Update, who);

        let (_, client_id, _) = PlayerBuilder::new().name("Ashur").build(&mut app);
        PlayerBuilder::new().name("Bau").build(&mut app);

        send_message(&mut app, client_id, "who");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Ashur, Bau");
    }
}
