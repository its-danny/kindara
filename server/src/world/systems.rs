use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_proto::prelude::*;
use futures_lite::future;
use sqlx::{types::Json, Pool, Postgres};

use crate::{
    db::{models::WorldSaveModel, pool::DatabasePool},
    items::components::{Inventory, Item},
    player::components::{Character, Online},
    spatial::components::Tile,
    value_or_continue,
};

use super::resources::{SaveTimer, WorldState, WorldStateCharacter};

pub fn spawn_trinus_castra(mut commands: ProtoCommands) {
    commands.spawn("world.trinus.trinus-castra");
}

pub fn spawn_the_roaring_lion(mut commands: ProtoCommands) {
    commands.spawn("world.trinus.the-roaring-lion");
}

#[derive(Component)]
pub struct SaveWorldTask(Task<Result<WorldState, sqlx::Error>>);

pub fn save_world_state(
    database: Res<DatabasePool>,
    inventories: Query<Option<&Children>, With<Inventory>>,
    items: Query<&Name, With<Item>>,
    mut bevy: Commands,
    mut save_timer: ResMut<SaveTimer>,
    players: Query<(&Character, &Parent, &Children), With<Online>>,
    tiles: Query<&Name, With<Tile>>,
    time: Res<Time>,
) {
    if save_timer.0.tick(time.delta()).just_finished() {
        let mut characters: Vec<WorldStateCharacter> = Vec::new();

        for (character, parent, children) in players.iter() {
            let tile_name =
                value_or_continue!(tiles.get(parent.get()).ok().map(|name| name.to_string()));

            let inventory = value_or_continue!(children
                .iter()
                .find_map(|child| inventories.get(*child).ok()));

            let items_names = inventory
                .iter()
                .flat_map(|children| children.iter())
                .filter_map(|child| items.get(*child).ok())
                .map(|name| name.to_string())
                .collect();

            let character = WorldStateCharacter {
                id: character.id,
                tile: tile_name,
                inventory: items_names,
            };

            characters.push(character);
        }

        bevy.spawn(SaveWorldTask(spawn_save_world_state_task(
            database.0.clone(),
            WorldState { characters },
        )));
    }
}

fn spawn_save_world_state_task(
    pool: Pool<Postgres>,
    state: WorldState,
) -> Task<Result<WorldState, sqlx::Error>> {
    AsyncComputeTaskPool::get().spawn(async move {
        let mut transaction = pool.begin().await?;

        let latest = sqlx::query_as::<_, WorldSaveModel>("SELECT * FROM world_saves ORDER BY id DESC LIMIT 1")
            .fetch_optional(&mut *transaction)
            .await?;

        let mut characters = state.characters.clone();

        if let Some(save) = latest {
            // To prevent offline characters from being removed from the world state,
            // we need to add them to the new state if they are not already present.
            for character in save.state.characters.iter() {
                if !state.characters.iter().any(|c| c.id == character.id) {
                    characters.push(character.clone());
                }
            }
        }

        let state = WorldState { characters };

        sqlx::query("INSERT INTO world_saves (state) VALUES ($1)")
            .bind(Json(&state))
            .execute(&mut *transaction)
            .await?;

        sqlx::query("DELETE FROM world_saves WHERE id IN (SELECT id FROM world_saves ORDER BY id ASC LIMIT 1) AND (SELECT COUNT(*) FROM world_saves) > 10")
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        Ok(state)
    })
}

pub fn handle_save_world_state_task(
    mut bevy: Commands,
    mut tasks: Query<(Entity, &mut SaveWorldTask)>,
    mut world_state: ResMut<WorldState>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(Ok(state)) = future::block_on(future::poll_once(&mut task.0)) {
            *world_state = state;

            bevy.entity(entity).remove::<SaveWorldTask>();
        }
    }
}

#[derive(Component)]
pub struct LoadWorldStateTask(Task<Result<WorldState, sqlx::Error>>);

pub fn load_world_state(mut bevy: Commands, database: Res<DatabasePool>) {
    bevy.spawn(LoadWorldStateTask(spawn_load_world_state_task(
        database.0.clone(),
    )));
}

fn spawn_load_world_state_task(pool: Pool<Postgres>) -> Task<Result<WorldState, sqlx::Error>> {
    AsyncComputeTaskPool::get().spawn(async move {
        let state = sqlx::query_as::<_, WorldSaveModel>(
            "SELECT * FROM world_saves ORDER BY id DESC LIMIT 1",
        )
        .fetch_one(&pool)
        .await?;

        Ok(state.state.0)
    })
}

pub fn handle_load_world_state_task(
    mut bevy: Commands,
    mut tasks: Query<(Entity, &mut LoadWorldStateTask)>,
    mut world_state: ResMut<WorldState>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(Ok(state)) = future::block_on(future::poll_once(&mut task.0)) {
            *world_state = state;

            bevy.entity(entity).remove::<LoadWorldStateTask>();
        }
    }
}
