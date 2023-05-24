use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_nest::prelude::*;
use futures_lite::future;
use owo_colors::OwoColorize;
use sqlx::{Pool, Postgres};

use crate::{
    db::{models::CharacterModel, pool::DatabasePool},
    player::{
        bundles::PlayerBundle,
        components::{Character, Client},
    },
    spatial::components::{Position, Zone},
};

use super::components::{AuthState, Authenticating};

// Entry point for the authentication process.
pub fn on_network_event(
    mut commands: Commands,
    mut events: EventReader<NetworkEvent>,
    mut outbox: EventWriter<Outbox>,
    clients: Query<(Entity, &Client)>,
) {
    for event in events.iter() {
        if let NetworkEvent::Connected(id) = event {
            // This will be the player entity from here on out.
            commands.spawn((Client(*id), Authenticating::default()));

            // Let the client know we support GMCP.
            outbox.send_command(*id, vec![IAC, WILL, GMCP]);

            outbox.send_text(
                *id,
                format!(
                    "Thou hast arrived in {}, wanderer. What name dost thou bear?",
                    "Aureus".bright_yellow()
                ),
            );
        }

        if let NetworkEvent::Disconnected(id) = event {
            if let Some((entity, _)) = clients.iter().find(|(_, c)| c.0 == *id) {
                commands.entity(entity).despawn();
            }
        }
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
    for message in inbox
        .iter()
        .filter(|m| matches!(m.content, Message::Text(_)))
    {
        let Some((client, mut auth)) = clients.iter_mut().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        match &mut auth.state {
            AuthState::Name => {
                if let Message::Text(name) = &message.content {
                    if name.len() < 3 || name.len() > 15 {
                        outbox.send_text(
                            client.0,
                            format!(
                                "{}",
                                "Name must be between 3 and 15 characters".bright_red()
                            ),
                        );

                        break;
                    }

                    auth.name = name.clone();
                    auth.state = AuthState::AwaitingTaskCompletion;

                    commands.spawn(UserExists(spawn_user_exists_task(
                        database.0.clone(),
                        client.0,
                        name.clone(),
                    )));
                }
            }
            AuthState::Password => {
                if let Message::Text(password) = &message.content {
                    if password.len() < 3 || password.len() > 15 {
                        outbox.send_text(
                            client.0,
                            format!(
                                "{}",
                                "Password must be between 3 and 15 characters".bright_red()
                            ),
                        );

                        break;
                    }

                    auth.state = AuthState::AwaitingTaskCompletion;

                    commands.spawn(Authenticated(spawn_authenticate_task(
                        database.0.clone(),
                        client.0,
                        auth.name.clone(),
                        password.clone(),
                    )));
                }
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
            let Some((_, mut auth)) = clients.iter_mut().find(|(c, _)| c.0 == client_id) else {
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
            outbox.send_command(client_id, vec![IAC, WILL, ECHO]);
            outbox.send_text(client_id, message);

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
                "INSERT INTO characters (name, password) VALUES ($1, $2) RETURNING *",
            )
            .bind(&name)
            .bind(&hashed)
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
                clients.iter_mut().find(|(_, c, _)| c.0 == client_id)
            else {
                return;
            };

            if let Some(character) = character_model {
                commands
                    .entity(player_entity)
                    .remove::<Authenticating>()
                    .insert(PlayerBundle {
                        character: Character {
                            name: character.name,
                            role: character.role,
                        },
                        position: Position {
                            zone: Zone::Movement,
                            coords: IVec3::ZERO,
                        },
                    });

                // Tell the client it's ok to resume echoing input.
                outbox.send_command(client_id, vec![IAC, WONT, ECHO]);
                outbox.send_text(
                    client.0,
                    format!("{}", "May thy journey here be prosperous.".bright_green()),
                );
            } else {
                auth.state = AuthState::Password;

                outbox.send_text(
                    client.0,
                    "The secret word thou hast given is not the right one.",
                );
            }

            commands.entity(task_entity).remove::<Authenticated>();
        }
    }
}
