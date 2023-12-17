use anyhow::Context;
use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use sqlx::{Pool, Postgres};

use crate::{
    auth::components::Authenticating,
    db::{pool::DatabasePool, utils::store_world_state},
    items::components::{Inventory, Item},
    player::components::{Character, Client, Online},
    spatial::components::Tile,
    world::resources::{WorldState, WorldStateCharacter},
};

use super::telnet::NAWS;

#[derive(Component)]
struct SaveCharacterTask(Task<Result<WorldState, sqlx::Error>>);

#[sysfail(log)]
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
) -> Result<(), anyhow::Error> {
    for event in events.iter() {
        if let NetworkEvent::Connected(id) = event {
            bevy.spawn((Client { id: *id, width: 80 }, Authenticating::default()));

            outbox.send_command(*id, vec![IAC, WILL, GMCP]);
            outbox.send_command(*id, vec![IAC, DO, NAWS]);

            outbox.send_text(
                *id,
                "You have arrived in Kindara, traveler. What name do you bear?",
            );
        }

        if let NetworkEvent::Disconnected(id) = event {
            if let Some((entity, _, character, parent, children)) =
                players.iter().find(|(_, c, _, _, _)| c.id == *id)
            {
                let tile = tiles
                    .get(parent.get())
                    .ok()
                    .map(|n| n.to_string())
                    .context("Tile not found")?;

                let inventory = children
                    .iter()
                    .find_map(|child| inventories.get(*child).ok())
                    .context("Inventory not found")?
                    .iter()
                    .flat_map(|children| children.iter())
                    .filter_map(|child| items.get(*child).ok())
                    .map(|(_, name)| name.to_string())
                    .collect::<Vec<_>>();

                let state = WorldStateCharacter {
                    id: character.id,
                    tile,
                    inventory,
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

    Ok(())
}

fn spawn_save_character_task(
    pool: Pool<Postgres>,
    state: WorldState,
) -> Task<Result<WorldState, sqlx::Error>> {
    AsyncComputeTaskPool::get().spawn(async move {
        let mut transaction = pool.begin().await?;

        store_world_state(&state, &mut transaction).await?;

        transaction.commit().await?;

        Ok(state)
    })
}
