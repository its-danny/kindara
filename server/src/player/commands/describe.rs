use std::sync::OnceLock;

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_nest::prelude::*;
use futures_lite::future;
use regex::Regex;
use sqlx::{Pool, Postgres};

use crate::{
    db::pool::DatabasePool,
    input::events::{Command, ParseError, ParsedCommand},
    player::components::{Character, Client, Online},
    value_or_continue,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_describe(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^describe( (?P<content>.*))?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let content = captures
                .name("content")
                .map(|m| m.as_str().trim().to_string());

            Ok(Command::Describe(content))
        }
    }
}

#[derive(Component)]
pub struct SaveDescriptionTask(Task<Result<ClientId, sqlx::Error>>);

pub fn describe(
    database: Res<DatabasePool>,
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut outbox: EventWriter<Outbox>,
    mut players: Query<(&Client, &mut Character), With<Online>>,
) {
    for command in commands.iter() {
        if let Command::Describe(content) = &command.command {
            let (client, mut character) =
                value_or_continue!(players.iter_mut().find(|(c, _)| c.id == command.from));

            if let Some(content) = content {
                character.description = Some(content.clone());

                bevy.spawn(SaveDescriptionTask(spawn_save_description_task(
                    database.0.clone(),
                    client.id,
                    character.id,
                    content.clone(),
                )));
            } else if let Some(description) = character.description.clone() {
                outbox.send_text(client.id, description);
            } else {
                outbox.send_text(client.id, "No description set.");
            }
        }
    }
}

fn spawn_save_description_task(
    pool: Pool<Postgres>,
    client_id: ClientId,
    character_id: i64,
    description: String,
) -> Task<Result<ClientId, sqlx::Error>> {
    AsyncComputeTaskPool::get().spawn(async move {
        sqlx::query("UPDATE characters SET description = $1 WHERE id = $2")
            .bind(&description)
            .bind(character_id)
            .execute(&pool)
            .await?;

        Ok(client_id)
    })
}

pub fn handle_save_description_task(
    mut bevy: Commands,
    mut tasks: Query<(Entity, &mut SaveDescriptionTask)>,
    mut outbox: EventWriter<Outbox>,
    players: Query<&Client, With<Online>>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(Ok(client_id)) = future::block_on(future::poll_once(&mut task.0)) {
            let client = value_or_continue!(players.iter().find(|c| c.id == client_id));

            outbox.send_text(client.id, "Description saved.");

            bevy.entity(entity).remove::<SaveDescriptionTask>();
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

    #[test]
    fn parses() {
        let content = handle_describe("describe A handsome fella.").unwrap();
        assert_eq!(content, Command::Describe(Some("A handsome fella.".into())));
    }

    #[sqlx::test]
    async fn valid(pool: PgPool) -> sqlx::Result<()> {
        let mut app = AppBuilder::new().database(&pool).build();
        app.add_systems(Update, (describe, handle_save_description_task));

        let (player, client_id, _) = PlayerBuilder::new().store(&pool).await?.build(&mut app);

        send_message(&mut app, client_id, "describe A handsome fella.");
        app.update();

        wait_for_task(&get_task::<SaveDescriptionTask>(&mut app).unwrap().0);
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Description saved.");
        assert_eq!(
            app.world.get::<Character>(player).unwrap().description,
            Some("A handsome fella.".into())
        );

        Ok(())
    }

    #[sqlx::test]
    async fn current_value(pool: PgPool) -> sqlx::Result<()> {
        let mut app = AppBuilder::new().database(&pool).build();
        app.add_systems(Update, (describe, handle_save_description_task));

        let (_, client_id, _) = PlayerBuilder::new()
            .description("A handsome fella.")
            .store(&pool)
            .await?
            .build(&mut app);

        send_message(&mut app, client_id, "describe");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "A handsome fella.");

        Ok(())
    }

    #[sqlx::test]
    async fn no_current_value(pool: PgPool) -> sqlx::Result<()> {
        let mut app = AppBuilder::new().database(&pool).build();
        app.add_systems(Update, (describe, handle_save_description_task));

        let (_, client_id, _) = PlayerBuilder::new().build(&mut app);

        send_message(&mut app, client_id, "describe");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "No description set.");

        Ok(())
    }
}
