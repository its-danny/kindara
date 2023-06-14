use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_nest::prelude::*;
use sqlx::{types::Json, Pool, Postgres};

use crate::{
    auth::components::Authenticating,
    db::pool::DatabasePool,
    items::components::{Inventory, Item},
    player::components::{Character, Client, Online},
    spatial::components::Tile,
    value_or_continue,
    world::resources::{WorldState, WorldStateCharacter},
};

use super::telnet::NAWS;

#[derive(Component)]
struct SaveCharacterTask(Task<Result<WorldState, sqlx::Error>>);

pub fn on_network_event(
    mut bevy: Commands,
    mut events: EventReader<NetworkEvent>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(Entity, &Client, &Character, &Parent, &Children), With<Online>>,
    database: Res<DatabasePool>,
    world_state: Res<WorldState>,
    inventories: Query<Option<&Children>, With<Inventory>>,
    items: Query<(Entity, &Name), With<Item>>,
    tiles: Query<&Name, With<Tile>>,
) {
    for event in events.iter() {
        if let NetworkEvent::Connected(id) = event {
            bevy.spawn((Client { id: *id, width: 80 }, Authenticating::default()));

            outbox.send_command(*id, vec![IAC, WILL, GMCP]);
            outbox.send_command(*id, vec![IAC, DO, NAWS]);

            outbox.send_text(
                *id,
                "Thou hast arrived in Aureus, wanderer. What name dost thou bear?",
            );
        }

        if let NetworkEvent::Disconnected(id) = event {
            if let Some((entity, _, character, parent, children)) =
                players.iter().find(|(_, c, _, _, _)| c.id == *id)
            {
                let tile = value_or_continue!(tiles.get(parent.get()).ok());
                let inventory = value_or_continue!(children
                    .iter()
                    .find_map(|child| inventories.get(*child).ok()));
                let items = inventory
                    .iter()
                    .flat_map(|children| children.iter())
                    .filter_map(|child| items.get(*child).ok())
                    .collect::<Vec<_>>();

                let state = WorldStateCharacter {
                    id: character.id,
                    tile: tile.to_string(),
                    inventory: items
                        .iter()
                        .map(|(_, name)| name.to_string())
                        .collect::<Vec<_>>(),
                };

                let mut characters = world_state.characters.clone();

                if let Some(index) = characters.iter().position(|c| c.id == character.id) {
                    characters[index] = state;
                } else {
                    characters.push(state);
                }

                let state = WorldState { characters };

                bevy.spawn(SaveCharacterTask(spawn_save_character_task(
                    database.0.clone(),
                    state,
                )));

                bevy.entity(entity).despawn();
            }
        }
    }
}

fn spawn_save_character_task(
    pool: Pool<Postgres>,
    state: WorldState,
) -> Task<Result<WorldState, sqlx::Error>> {
    AsyncComputeTaskPool::get().spawn(async move {
        let mut transaction = pool.begin().await?;

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
