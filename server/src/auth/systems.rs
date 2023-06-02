use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_nest::prelude::*;
use censor::Censor;
use futures_lite::future;
use regex::Regex;
use sqlx::{types::Json, Pool, Postgres};

use crate::{
    db::{models::CharacterModel, pool::DatabasePool},
    player::{
        bundles::PlayerBundle,
        components::{Character, Client},
        config::CharacterConfig,
    },
    spatial::components::{Position, Zone},
    text_messages,
};

use super::components::{AuthState, Authenticating};

fn name_is_valid(name: &str) -> Result<(), &'static str> {
    if name.len() < 3 || name.len() > 25 {
        return Err("Name must be between 3 and 25 characters.");
    }

    let regex = Regex::new(r"^[a-zA-Z]+(\s[a-zA-Z]+)?$").unwrap();

    if !regex.is_match(name) {
        return Err("Name must be alphanumeric.");
    }

    let ban_list = Censor::custom(vec!["admin", "mod", "moderator", "gm", "god", "immortal"]);
    let censor = Censor::Standard + Censor::Sex + ban_list;

    if censor.check(name) {
        return Err("Name contains banned words.");
    }

    Ok(())
}

fn password_is_valid(password: &str) -> Result<(), &'static str> {
    if password.len() >= 3 && password.len() <= 30 {
        Ok(())
    } else {
        Err("Password must be between 3 and 30 characters.")
    }
}

// Due to the async nature of database queries, we need to use AsyncComputeTaskPool to spawn tasks
// so as to not block the main thread. Those tasks are then handled via the handle_*_task systems below.
pub fn authenticate(
    mut commands: Commands,
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    database: Res<DatabasePool>,
    mut clients: Query<(&Client, &mut Authenticating)>,
) {
    for (message, content) in text_messages!(inbox) {
        let Some((client, mut auth)) = clients.iter_mut().find(|(c, _)| c.id == message.from) else {
            return;
        };

        match &mut auth.state {
            AuthState::Name => {
                if let Err(err) = name_is_valid(content) {
                    outbox.send_text(client.id, err);

                    break;
                }

                auth.name = content.clone();
                auth.state = AuthState::AwaitingTaskCompletion;

                commands.spawn(UserExists(spawn_user_exists_task(
                    database.0.clone(),
                    client.id,
                    content.clone(),
                )));
            }
            AuthState::Password => {
                if let Err(err) = password_is_valid(content) {
                    outbox.send_text(client.id, err);

                    break;
                }

                auth.state = AuthState::AwaitingTaskCompletion;

                commands.spawn(Authenticated(spawn_authenticate_task(
                    database.0.clone(),
                    client.id,
                    auth.name.clone(),
                    content.clone(),
                )));
            }
            AuthState::AwaitingTaskCompletion => {
                break;
            }
        }
    }
}

#[derive(Component)]
pub struct UserExists(Task<Result<(bool, ClientId), sqlx::Error>>);

// See `handle_user_exists_task` for the next step in the authentication process.
fn spawn_user_exists_task(
    pool: Pool<Postgres>,
    client_id: ClientId,
    name: String,
) -> Task<Result<(bool, ClientId), sqlx::Error>> {
    AsyncComputeTaskPool::get().spawn(async move {
        let (exists,): (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM characters WHERE name = $1)")
                .bind(&name)
                .fetch_one(&pool)
                .await?;

        Ok((exists, client_id))
    })
}

// This system handles the result of `spawn_user_exists_task`.
pub fn handle_user_exists_task(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut UserExists)>,
    mut outbox: EventWriter<Outbox>,
    mut clients: Query<(&Client, &mut Authenticating)>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(Ok((exists, client_id))) = future::block_on(future::poll_once(&mut task.0)) {
            let Some((client, mut auth)) = clients.iter_mut().find(|(c, _)| c.id == client_id) else {
                return;
            };

            auth.state = AuthState::Password;

            let message = if exists {
                format!(
                    "Hail, returned {}! What is the secret word thou dost keep?",
                    auth.name
                )
            } else {
                format!("Hail, {}. Set for thyself a word of secrecy.", auth.name)
            };

            // Tell the client to stop echoing input.
            outbox.send_command(client.id, vec![IAC, WILL, ECHO]);
            outbox.send_text(client.id, message);

            commands.entity(entity).remove::<UserExists>();
        }
    }
}

#[derive(Component)]
pub struct Authenticated(Task<Result<(Option<CharacterModel>, ClientId), sqlx::Error>>);

// See `handle_authenticate_task` for the next step in the authentication process.
fn spawn_authenticate_task(
    pool: Pool<Postgres>,
    client_id: ClientId,
    name: String,
    password: String,
) -> Task<Result<(Option<CharacterModel>, ClientId), sqlx::Error>> {
    AsyncComputeTaskPool::get().spawn(async move {
        let character =
            sqlx::query_as::<_, CharacterModel>("SELECT * FROM characters WHERE name = $1")
                .bind(&name)
                .fetch_optional(&pool)
                .await?;

        if let Some(character) = character {
            bcrypt::verify(&password, &character.password)
                .map(|verified| if verified { Some(character) } else { None })
                .map_or_else(
                    |_| Ok((None, client_id)),
                    |character| Ok((character, client_id)),
                )
        } else if let Ok(hashed) = bcrypt::hash(&password, bcrypt::DEFAULT_COST) {
            let character = sqlx::query_as::<_, CharacterModel>(
                "INSERT INTO characters (name, password, config) VALUES ($1, $2, $3) RETURNING *",
            )
            .bind(&name)
            .bind(&hashed)
            .bind(Json(CharacterConfig::default()))
            .fetch_one(&pool)
            .await?;

            Ok((Some(character), client_id))
        } else {
            Ok((None, client_id))
        }
    })
}

// This system handles the result of `spawn_authenticate_task`.
pub fn handle_authenticate_task(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut Authenticated)>,
    mut clients: Query<(Entity, &Client, &mut Authenticating)>,
    mut outbox: EventWriter<Outbox>,
) {
    for (task_entity, mut task) in &mut tasks {
        if let Some(Ok((character_model, client_id))) =
            future::block_on(future::poll_once(&mut task.0))
        {
            let Some((player_entity, client, mut auth)) =
                clients.iter_mut().find(|(_, c, _)| c.id == client_id)
            else {
                return;
            };

            if let Some(character) = character_model {
                commands
                    .entity(player_entity)
                    .remove::<Authenticating>()
                    .insert(PlayerBundle {
                        character: Character {
                            id: character.id,
                            name: character.name,
                            role: character.role,
                            config: character.config.0,
                        },
                        position: Position {
                            zone: Zone::Movement,
                            coords: IVec3::ZERO,
                        },
                    });

                // Tell the client it's ok to resume echoing input.
                outbox.send_command(client.id, vec![IAC, WONT, ECHO]);
                outbox.send_text(client.id, "May thy journey here be prosperous.");
            } else {
                auth.state = AuthState::Password;

                outbox.send_text(
                    client.id,
                    "The secret word thou hast given is not the right one.",
                );
            }

            commands.entity(task_entity).remove::<Authenticated>();
        }
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        utils::{get_command_content, get_message_content, get_task, send_message, wait_for_task},
    };

    use super::*;

    #[test]
    fn is_name_valid() {
        assert!(name_is_valid("Caesar").is_ok());
        assert!(name_is_valid("Caesar Augustus").is_ok());
        assert!(name_is_valid("Caesar   Augustus").is_err());
        assert!(name_is_valid("Caesar Octavian Augustus").is_err());
        assert!(name_is_valid("shit god").is_err());
        assert!(name_is_valid("admin").is_err());
    }

    #[test]
    fn is_password_valid() {
        assert!(password_is_valid("no").is_err());
        assert!(password_is_valid("hippopotomonstrosesquippedaliophobia").is_err());
        assert!(password_is_valid("password").is_ok());
    }

    #[sqlx::test]
    async fn new_character(pool: PgPool) -> sqlx::Result<()> {
        let mut app = AppBuilder::new().database(&pool).build();

        app.add_systems((
            authenticate,
            handle_user_exists_task,
            handle_authenticate_task,
        ));

        let (client_id, player) = PlayerBuilder::new()
            .authenticating(true)
            .name("Icauna")
            .build(&mut app);

        send_message(&mut app, client_id, "Icauna");
        app.update();

        wait_for_task(&get_task::<UserExists>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id);
        assert_eq!(command, vec![IAC, WILL, ECHO]);

        let content = get_message_content(&mut app, client_id);
        assert_eq!(content, "Hail, Icauna. Set for thyself a word of secrecy.");

        send_message(&mut app, client_id, "secret");
        app.update();

        wait_for_task(&get_task::<Authenticated>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id);
        assert_eq!(command, vec![IAC, WONT, ECHO]);

        let content = get_message_content(&mut app, client_id);
        assert_eq!(content, "May thy journey here be prosperous.");

        assert!(app.world.get::<Authenticating>(player).is_none());
        assert!(app.world.get::<Character>(player).is_some());

        let (exists,): (bool,) =
            sqlx::query_as("SELECT EXISTS(SELECT 1 FROM characters WHERE name = $1)")
                .bind("Icauna")
                .fetch_one(&pool)
                .await?;

        assert!(exists);

        Ok(())
    }

    #[sqlx::test]
    fn existing_character(pool: PgPool) -> sqlx::Result<()> {
        let mut app = AppBuilder::new().database(&pool).build();

        app.add_systems((
            authenticate,
            handle_user_exists_task,
            handle_authenticate_task,
        ));

        let (client_id, player) = PlayerBuilder::new()
            .authenticating(true)
            .name("Bres")
            .password("secret")
            .store(&pool)
            .await?
            .build(&mut app);

        send_message(&mut app, client_id, "Bres");
        app.update();

        wait_for_task(&get_task::<UserExists>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id);
        assert_eq!(command, vec![IAC, WILL, ECHO]);

        let content = get_message_content(&mut app, client_id);
        assert_eq!(
            content,
            "Hail, returned Bres! What is the secret word thou dost keep?"
        );

        send_message(&mut app, client_id, "secret");
        app.update();

        wait_for_task(&get_task::<Authenticated>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id);
        assert_eq!(command, vec![IAC, WONT, ECHO]);

        let content = get_message_content(&mut app, client_id);
        assert_eq!(content, "May thy journey here be prosperous.");

        assert!(app.world.get::<Authenticating>(player).is_none());
        assert!(app.world.get::<Character>(player).is_some());

        Ok(())
    }

    #[sqlx::test]
    fn wrong_password(pool: PgPool) -> sqlx::Result<()> {
        let mut app = AppBuilder::new().database(&pool).build();

        app.add_systems((
            authenticate,
            handle_user_exists_task,
            handle_authenticate_task,
        ));

        let (client_id, player) = PlayerBuilder::new()
            .authenticating(true)
            .name("Bres")
            .password("secret")
            .store(&pool)
            .await?
            .build(&mut app);

        send_message(&mut app, client_id, "Bres");
        app.update();

        wait_for_task(&get_task::<UserExists>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id);
        assert_eq!(command, vec![IAC, WILL, ECHO]);

        let content = get_message_content(&mut app, client_id);
        assert_eq!(
            content,
            "Hail, returned Bres! What is the secret word thou dost keep?"
        );

        send_message(&mut app, client_id, "wrong");
        app.update();

        wait_for_task(&get_task::<Authenticated>(&mut app).unwrap().0);
        app.update();

        let content = get_message_content(&mut app, client_id);
        assert_eq!(
            content,
            "The secret word thou hast given is not the right one."
        );

        assert!(app.world.get::<Authenticating>(player).is_some());
        assert!(app.world.get::<Authenticating>(player).unwrap().state == AuthState::Password);
        assert!(app.world.get::<Character>(player).is_none());

        Ok(())
    }
}
