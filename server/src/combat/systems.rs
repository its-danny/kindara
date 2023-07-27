use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{
    combat::components::State,
    npc::components::Npc,
    player::components::{Character, CharacterState, Client, Online},
    spatial::components::Tile,
    visual::components::Depiction,
};

use super::components::HasAttacked;

pub fn update_attack_timer(
    mut bevy: Commands,
    mut timers: Query<(Entity, &mut HasAttacked)>,
    time: Res<Time>,
) {
    for (entity, mut has_attacked) in timers.iter_mut() {
        has_attacked.timer.tick(time.delta());

        if has_attacked.timer.finished() {
            bevy.entity(entity).remove::<HasAttacked>();
        }
    }
}

pub fn on_npc_death(
    mut bevy: Commands,
    mut outbox: EventWriter<Outbox>,
    npcs: Query<(Entity, &Depiction, &State, &Parent), With<Npc>>,
    mut players: Query<(&Client, &mut Character), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) {
    for (entity, depiction, state, parent) in npcs.iter() {
        let siblings = tiles.get(parent.get()).ok();

        if state.health == 0 {
            let players_in_combat = players
                .iter_mut()
                .filter(|(_, character)| character.state == CharacterState::Combat(entity));

            for (_, mut character) in players_in_combat {
                character.state = CharacterState::Idle;
            }

            let players_on_tile = siblings
                .map(|siblings| {
                    siblings
                        .iter()
                        .filter_map(|entity| players.get(*entity).ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            for (client, _) in players_on_tile {
                outbox.send_text(client.id, format!("{} has died.", depiction.name));
            }

            bevy.entity(entity).despawn();
        }
    }
}
