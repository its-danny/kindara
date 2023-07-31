use std::sync::OnceLock;

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_nest::prelude::*;
use bevy_proto::prelude::ProtoCommands;
use censor::Censor;
use futures_lite::future;
use regex::Regex;
use sqlx::{types::Json, Pool, Postgres};

use crate::{
    combat::{
        bundles::CombatBundle,
        components::{Attributes, State},
    },
    db::{
        models::{CharacterModel, Role},
        pool::DatabasePool,
    },
    input::events::{Command, ParsedCommand, ProxyCommand},
    items::components::Inventory,
    keycard::Keycard,
    paint,
    player::{
        bundles::PlayerBundle,
        components::{Character, CharacterState, Client, Online},
        config::CharacterConfig,
    },
    spatial::components::{Spawn, Tile},
    value_or_continue,
    world::resources::WorldState,
};

use super::components::{AuthState, Authenticating};

static NAME_REGEX: OnceLock<Regex> = OnceLock::new();

#[derive(Component)]
pub struct UserExistsTask(Task<Result<(bool, ClientId), sqlx::Error>>);

#[derive(Component)]
pub struct AuthenticateTask(Task<Result<(Option<CharacterModel>, ClientId), sqlx::Error>>);

pub fn authenticate(
    database: Res<DatabasePool>,
    mut bevy: Commands,
    mut clients: Query<(&Client, &mut Authenticating), Without<Online>>,
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
) {
    for (message, content) in inbox.iter().filter_map(|m| {
        if let Message::Text(content) = &m.content {
            Some((m, content))
        } else {
            None
        }
    }) {
        let (client, mut auth) =
            value_or_continue!(clients.iter_mut().find(|(c, _)| c.id == message.from));

        match &mut auth.state {
            AuthState::Name => {
                if let Err(err) = name_is_valid(content) {
                    outbox.send_text(client.id, paint!("<fg.red>{err}</>"));

                    continue;
                }

                auth.name = content.clone();
                auth.state = AuthState::AwaitingTaskCompletion;

                bevy.spawn(UserExistsTask(spawn_user_exists_task(
                    database.0.clone(),
                    client.id,
                    content.clone(),
                )));
            }
            AuthState::Password => {
                if let Err(err) = password_is_valid(content) {
                    outbox.send_text(client.id, paint!("<fg.red>{err}</>"));

                    continue;
                }

                auth.state = AuthState::AwaitingTaskCompletion;

                bevy.spawn(AuthenticateTask(spawn_authenticate_task(
                    database.0.clone(),
                    client.id,
                    auth.name.clone(),
                    content.clone(),
                )));
            }
            AuthState::AwaitingTaskCompletion => {}
        }
    }
}

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

pub fn handle_user_exists_task(
    mut bevy: Commands,
    mut tasks: Query<(Entity, &mut UserExistsTask)>,
    mut outbox: EventWriter<Outbox>,
    mut clients: Query<(&Client, &mut Authenticating), Without<Online>>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(Ok((exists, client_id))) = future::block_on(future::poll_once(&mut task.0)) {
            let (client, mut auth) =
                value_or_continue!(clients.iter_mut().find(|(c, _)| c.id == client_id));

            auth.state = AuthState::Password;

            outbox.send_command(client.id, vec![IAC, WILL, ECHO]);
            outbox.send_text(
                client.id,
                if exists {
                    format!("Hail, {}! What is the secret word you keep?", auth.name)
                } else {
                    format!("Hail, {}. Set for yourself a word of secrecy.", auth.name)
                },
            );

            bevy.entity(entity).remove::<UserExistsTask>();
        }
    }
}

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

pub fn handle_authenticate_task(
    mut bevy: Commands,
    mut clients: Query<(Entity, &Client, &mut Authenticating), Without<Online>>,
    mut outbox: EventWriter<Outbox>,
    mut proto: ProtoCommands,
    mut proxy: EventWriter<ProxyCommand>,
    mut tasks: Query<(Entity, &mut AuthenticateTask)>,
    online_characters: Query<(&Client, &Character), With<Online>>,
    server: Res<Server>,
    spawn_tiles: Query<Entity, (With<Tile>, With<Spawn>)>,
    tiles: Query<(Entity, &Name), With<Tile>>,
    world_state: Res<WorldState>,
) {
    for (task_entity, mut task) in &mut tasks {
        if let Some(Ok((character_model, client_id))) =
            future::block_on(future::poll_once(&mut task.0))
        {
            let (player_entity, client, mut auth) =
                value_or_continue!(clients.iter_mut().find(|(_, c, _)| c.id == client_id));

            if let Some(character) = character_model {
                if let Some((online, _)) =
                    online_characters.iter().find(|(_, c)| c.id == character.id)
                {
                    outbox.send_text(
                        client.id,
                        paint!(
                            "<fg.player>{}</> is already online and will be disconnected.",
                            character.name
                        ),
                    );

                    server.disconnect(&online.id);
                }

                bevy.entity(player_entity)
                    .remove::<Authenticating>()
                    .insert((
                        Online,
                        PlayerBundle {
                            keycard: match character.role {
                                Role::Admin => Keycard::admin(),
                                Role::Player => Keycard::player(),
                            },
                            character: Character {
                                id: character.id,
                                name: character.name,
                                description: character.description,
                                config: character.config.0,
                                state: CharacterState::Idle,
                            },
                        },
                        CombatBundle {
                            attributes: Attributes::default(),
                        },
                        State {
                            health: Attributes::default().max_health(),
                        },
                    ));

                let spawn = value_or_continue!(spawn_tiles.iter().next());

                let character_in_state =
                    world_state.characters.iter().find(|c| c.id == character.id);

                if let Some(character_in_state) = character_in_state {
                    let tile = tiles
                        .iter()
                        .find(|(_, name)| {
                            name.trim_end_matches(" (Prototype)")
                                == character_in_state.tile.trim_end_matches(" (Prototype)")
                        })
                        .map(|(e, _)| e)
                        .unwrap_or(spawn);

                    bevy.entity(player_entity)
                        .set_parent(tile)
                        .with_children(|parent| {
                            let mut inventory = parent.spawn(Inventory);

                            for item_name in character_in_state.inventory.iter() {
                                inventory.add_child(
                                    proto.spawn(item_name.trim_end_matches(" (Prototype)")).id(),
                                );
                            }
                        });
                } else {
                    bevy.entity(player_entity)
                        .set_parent(spawn)
                        .with_children(|parent| {
                            parent.spawn(Inventory);
                        });
                }

                outbox.send_command(client.id, vec![IAC, WONT, ECHO]);
                outbox.send_text(client.id, "May your journey here be prosperous.");

                proxy.send(ProxyCommand(ParsedCommand {
                    from: client.id,
                    command: Command::Look(None),
                }));
            } else {
                auth.state = AuthState::Password;

                outbox.send_text(
                    client.id,
                    "The secret word you have given is not the right one.",
                );
            }

            bevy.entity(task_entity).remove::<AuthenticateTask>();
        }
    }
}

fn name_is_valid(name: &str) -> Result<(), &'static str> {
    if name.len() < 3 || name.len() > 25 {
        return Err("Name must be between 3 and 25 characters.");
    }

    let regex = NAME_REGEX.get_or_init(|| Regex::new(r"^[a-zA-Z]+(\s[a-zA-Z]+)?$").unwrap());

    if !regex.is_match(name) {
        return Err("Name can only contain letters and spaces.");
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

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use crate::test::{
        app_builder::AppBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
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

        app.add_systems(
            Update,
            (
                authenticate,
                handle_user_exists_task,
                handle_authenticate_task,
            ),
        );

        let zone = ZoneBuilder::new().build(&mut app);
        TileBuilder::new().is_spawn().build(&mut app, zone);

        let (player, client_id, _) = PlayerBuilder::new()
            .is_authenticating()
            .name("Icauna")
            .build(&mut app);

        send_message(&mut app, client_id, "Icauna");
        app.update();

        wait_for_task(&get_task::<UserExistsTask>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id).unwrap();
        assert_eq!(command, vec![IAC, WILL, ECHO]);

        let content = get_message_content(&mut app, client_id).unwrap();
        assert_eq!(content, "Hail, Icauna. Set for yourself a word of secrecy.");

        send_message(&mut app, client_id, "secret");
        app.update();

        wait_for_task(&get_task::<AuthenticateTask>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id).unwrap();
        assert_eq!(command, vec![IAC, WONT, ECHO]);

        let content = get_message_content(&mut app, client_id).unwrap();
        assert_eq!(content, "May your journey here be prosperous.");

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

        app.add_systems(
            Update,
            (
                authenticate,
                handle_user_exists_task,
                handle_authenticate_task,
            ),
        );

        let zone = ZoneBuilder::new().build(&mut app);
        TileBuilder::new().is_spawn().build(&mut app, zone);

        let (player, client_id, _) = PlayerBuilder::new()
            .is_authenticating()
            .name("Bres")
            .password("secret")
            .store(&pool)
            .await?
            .build(&mut app);

        send_message(&mut app, client_id, "Bres");
        app.update();

        wait_for_task(&get_task::<UserExistsTask>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id).unwrap();
        assert_eq!(command, vec![IAC, WILL, ECHO]);

        let content = get_message_content(&mut app, client_id).unwrap();
        assert_eq!(content, "Hail, Bres! What is the secret word you keep?");

        send_message(&mut app, client_id, "secret");
        app.update();

        wait_for_task(&get_task::<AuthenticateTask>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id).unwrap();
        assert_eq!(command, vec![IAC, WONT, ECHO]);

        let content = get_message_content(&mut app, client_id).unwrap();
        assert_eq!(content, "May your journey here be prosperous.");

        assert!(app.world.get::<Authenticating>(player).is_none());
        assert!(app.world.get::<Character>(player).is_some());

        Ok(())
    }

    #[sqlx::test]
    fn wrong_password(pool: PgPool) -> sqlx::Result<()> {
        let mut app = AppBuilder::new().database(&pool).build();

        app.add_systems(
            Update,
            (
                authenticate,
                handle_user_exists_task,
                handle_authenticate_task,
            ),
        );

        let (player, client_id, _) = PlayerBuilder::new()
            .is_authenticating()
            .name("Bres")
            .password("secret")
            .store(&pool)
            .await?
            .build(&mut app);

        send_message(&mut app, client_id, "Bres");
        app.update();

        wait_for_task(&get_task::<UserExistsTask>(&mut app).unwrap().0);
        app.update();

        let command = get_command_content(&mut app, client_id).unwrap();
        assert_eq!(command, vec![IAC, WILL, ECHO]);

        let content = get_message_content(&mut app, client_id).unwrap();
        assert_eq!(content, "Hail, Bres! What is the secret word you keep?");

        send_message(&mut app, client_id, "wrong");
        app.update();

        wait_for_task(&get_task::<AuthenticateTask>(&mut app).unwrap().0);
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();
        assert_eq!(
            content,
            "The secret word you have given is not the right one."
        );

        assert!(app.world.get::<Authenticating>(player).is_some());
        assert!(app.world.get::<Authenticating>(player).unwrap().state == AuthState::Password);
        assert!(app.world.get::<Character>(player).is_none());

        Ok(())
    }
}
