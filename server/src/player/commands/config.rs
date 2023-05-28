use std::fmt::Display;

use ascii_table::AsciiTable;
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_nest::prelude::*;
use futures_lite::future;
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::{types::Json, Pool, Postgres};

use crate::{
    db::pool::DatabasePool,
    input::events::{Command, ParsedCommand},
    player::{
        components::{Character, Client},
        config::CharacterConfig,
    },
};

static REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^config(?:\s+(?P<option>\S+))?(?:\s+(?P<value>.*))?$").unwrap());

pub fn parse_config(
    client: &Client,
    content: &str,
    commands: &mut EventWriter<ParsedCommand>,
) -> bool {
    if let Some(captures) = REGEX.captures(content) {
        let option = captures.name("option").map(|m| m.as_str().to_string());
        let value = captures.name("value").map(|m| m.as_str().to_string());

        commands.send(ParsedCommand {
            from: client.id,
            command: Command::Config((option, value)),
        });

        true
    } else {
        false
    }
}

pub fn config(
    database: Res<DatabasePool>,
    mut bevy_commands: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Character)>,
) {
    for command in commands.iter() {
        if let Command::Config((option, value)) = &command.command {
            let Some((client, mut character)) = players.iter_mut().find(|(c, _)| c.id == command.from) else {
                return;
            };

            match (option, value) {
                (None, None) => {
                    let mut table = AsciiTable::default();
                    table.set_max_width(64);
                    table.column(0).set_header("config");
                    table.column(1).set_header("options");
                    table.column(2).set_header("value");

                    let options: Vec<Vec<&dyn Display>> =
                        vec![vec![&"brief", &"<true|false>", &character.config.brief]];

                    outbox.send_text(client.id, table.format(options));
                }
                (Some(option), None) => {
                    if let Some((description, current_value)) = character.config.get(option) {
                        outbox.send_text(
                            client.id,
                            format!("{description}\nCurrent value: {current_value}"),
                        );
                    } else {
                        outbox.send_text(client.id, format!("Unknown option: {option}"));
                    }
                }
                (Some(option), Some(value)) => match character.config.set(option, value) {
                    Ok(_) => {
                        bevy_commands.spawn(SaveConfig(spawn_save_config_task(
                            database.0.clone(),
                            client.id,
                            character.id,
                            character.config,
                        )));
                    }
                    Err(err) => outbox.send_text(client.id, err),
                },
                _ => {}
            }
        }
    }
}

#[derive(Component)]
pub struct SaveConfig(Task<Result<ClientId, sqlx::Error>>);

fn spawn_save_config_task(
    pool: Pool<Postgres>,
    client_id: ClientId,
    character_id: i64,
    config: CharacterConfig,
) -> Task<Result<ClientId, sqlx::Error>> {
    AsyncComputeTaskPool::get().spawn(async move {
        sqlx::query("UPDATE characters SET config = $1 WHERE id = $2")
            .bind(Json(config))
            .bind(character_id)
            .execute(&pool)
            .await?;

        Ok(client_id)
    })
}

pub fn handle_save_config_task(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut SaveConfig)>,
    mut outbox: EventWriter<Outbox>,
    players: Query<&Client, With<Character>>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(Ok(client_id)) = future::block_on(future::poll_once(&mut task.0)) {
            let Some(client) = players.iter().find(|c| c.id == client_id) else {
                return;
            };

            outbox.send_text(client.id, "Config saved.");

            commands.entity(entity).remove::<SaveConfig>();
        }
    }
}
