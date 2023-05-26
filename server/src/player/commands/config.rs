use std::fmt::Display;

use ascii_table::AsciiTable;
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_nest::prelude::*;
use futures_lite::future;
use regex::Regex;
use sqlx::{types::Json, Pool, Postgres};

use crate::{
    db::pool::DatabasePool,
    player::{
        components::{Character, Client},
        config::CharacterConfig,
    },
};

// USAGE: config [option] [value>]
pub fn config(
    database: Res<DatabasePool>,
    mut commands: Commands,
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Character)>,
) {
    let regex = Regex::new(r"^config(?:\s+(?P<option>\S+))?(?:\s+(?P<value>.*))?$").unwrap();

    for (message, captures) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) => regex.captures(text).map(|caps| (message, caps)),
        _ => None,
    }) {
        let Some((client, mut character)) = players.iter_mut().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        let option = captures.name("option").map(|m| m.as_str());
        let value = captures.name("value").map(|m| m.as_str());

        match (option, value) {
            (None, None) => {
                let mut table = AsciiTable::default();
                table.set_max_width(64);
                table.column(0).set_header("config");
                table.column(1).set_header("options");
                table.column(2).set_header("value");

                let options: Vec<Vec<&dyn Display>> =
                    vec![vec![&"brief", &"<true|false>", &character.config.brief]];

                outbox.send_text(client.0, table.format(options));
            }
            (Some(option), None) => {
                if let Some((description, current_value)) = character.config.get(option) {
                    outbox.send_text(
                        client.0,
                        format!("{description}\nCurrent value: {current_value}"),
                    );
                } else {
                    outbox.send_text(client.0, format!("Unknown option: {option}"));
                }
            }
            (Some(option), Some(value)) => match character.config.set(option, value) {
                Ok(_) => {
                    commands.spawn(SaveConfig(spawn_save_config_task(
                        database.0.clone(),
                        client.0,
                        character.id,
                        character.config,
                    )));
                }
                Err(err) => outbox.send_text(client.0, err),
            },
            _ => {}
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
            let Some(client) = players.iter().find(|c| c.0 == client_id) else {
                return;
            };

            outbox.send_text(client.0, "Config saved.");

            commands.entity(entity).remove::<SaveConfig>();
        }
    }
}
