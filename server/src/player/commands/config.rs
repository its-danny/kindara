use std::{fmt::Display, sync::OnceLock};

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
    input::events::{Command, ParseError, ParsedCommand},
    player::{
        components::{Character, Client, Online},
        config::CharacterConfig,
    },
    value_or_continue,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_config(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| {
        Regex::new(r"^config(?:\s+(?P<option>\S+))?(?:\s+(?P<value>.*))?$").unwrap()
    });

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let option = captures.name("option").map(|m| m.as_str().to_string());
            let value = captures.name("value").map(|m| m.as_str().to_string());

            Ok(Command::Config((option, value)))
        }
    }
}

#[derive(Component)]
pub struct SaveConfigTask(Task<Result<ClientId, sqlx::Error>>);

pub fn config(
    database: Res<DatabasePool>,
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Character), With<Online>>,
) {
    for command in commands.iter() {
        if let Command::Config((option, value)) = &command.command {
            let (client, mut character) =
                value_or_continue!(players.iter_mut().find(|(c, _)| c.id == command.from));

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
                (Some(option), None) => match character.config.get(option) {
                    Ok((current_value, description)) => {
                        outbox.send_text(
                            client.id,
                            format!("{description}\nCurrent value: {current_value}"),
                        );
                    }
                    Err(err) => {
                        outbox.send_text(client.id, err);
                    }
                },
                (Some(option), Some(value)) => match character.config.set(option, value) {
                    Ok(_) => {
                        bevy.spawn(SaveConfigTask(spawn_save_config_task(
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
    mut bevy: Commands,
    mut tasks: Query<(Entity, &mut SaveConfigTask)>,
    mut outbox: EventWriter<Outbox>,
    players: Query<&Client, With<Online>>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(Ok(client_id)) = future::block_on(future::poll_once(&mut task.0)) {
            let client = value_or_continue!(players.iter().find(|c| c.id == client_id));

            outbox.send_text(client.id, "Config saved.");

            bevy.entity(entity).remove::<SaveConfigTask>();
        }
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        utils::{get_message_content, get_task, send_message, wait_for_task},
    };

    use super::*;

    #[sqlx::test]
    async fn valid(pool: PgPool) -> sqlx::Result<()> {
        let mut app = AppBuilder::new().database(&pool).build();
        app.add_systems(Update, (config, handle_save_config_task));

        let (player, client_id, _) = PlayerBuilder::new()
            .config(CharacterConfig {
                brief: false,
                ..Default::default()
            })
            .store(&pool)
            .await?
            .build(&mut app);

        send_message(&mut app, client_id, "config brief true");
        app.update();

        assert_eq!(
            app.world.get::<Character>(player).unwrap().config.brief,
            true
        );

        wait_for_task(&get_task::<SaveConfigTask>(&mut app).unwrap().0);
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Config saved.");

        Ok(())
    }

    #[sqlx::test]
    async fn current_value(pool: PgPool) -> sqlx::Result<()> {
        let mut app = AppBuilder::new().database(&pool).build();
        app.add_systems(Update, config);

        let (_, client_id, _) = PlayerBuilder::new()
            .config(CharacterConfig {
                brief: false,
                ..Default::default()
            })
            .build(&mut app);

        send_message(&mut app, client_id, "config brief");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert!(content.contains("Current value: false"));

        Ok(())
    }

    #[sqlx::test]
    fn invalid_option(pool: PgPool) {
        let mut app = AppBuilder::new().database(&pool).build();
        app.add_systems(Update, config);

        let (_, client_id, _) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "config god true");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Invalid option.");
    }

    #[sqlx::test]
    fn invalid_value(pool: PgPool) {
        let mut app = AppBuilder::new().database(&pool).build();
        app.add_systems(Update, config);

        let (_, client_id, _) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "config brief please");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Value must be `true` or `false`.");
    }
}
