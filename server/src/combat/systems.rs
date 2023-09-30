use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{
    combat::components::State,
    input::events::{Command, ParsedCommand, ProxyCommand},
    npc::components::Npc,
    player::components::{Client, Online},
    spatial::components::{DeathSpawn, Tile},
    visual::components::Depiction,
};

use super::components::{Attributes, HasAttacked, InCombat, QueuedAttack};

pub fn update_attack_timer(
    mut bevy: Commands,
    mut proxy: EventWriter<ProxyCommand>,
    mut timers: Query<(
        Entity,
        &mut HasAttacked,
        Option<&QueuedAttack>,
        Option<&Client>,
    )>,
    time: Res<Time>,
    mut outbox: EventWriter<Outbox>,
) {
    for (entity, mut has_attacked, queued_attack, client) in timers.iter_mut() {
        has_attacked.timer.tick(time.delta());

        if has_attacked.timer.finished() {
            bevy.entity(entity).remove::<HasAttacked>();

            match queued_attack {
                Some(queued_attack) => {
                    proxy.send(ProxyCommand(queued_attack.0.clone()));
                    bevy.entity(entity).remove::<QueuedAttack>();
                }
                None => {
                    if let Some(client) = client {
                        outbox.send_text(client.id, "You are ready attack again.");
                    }
                }
            }
        }
    }
}

pub fn on_npc_death(
    mut bevy: Commands,
    mut outbox: EventWriter<Outbox>,
    npcs: Query<(Entity, &Depiction, &State, &Parent), With<Npc>>,
    mut players: Query<(Entity, &Client, &InCombat), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) {
    for (entity, depiction, state, parent) in npcs.iter() {
        let siblings = tiles.get(parent.get()).ok();

        if state.health == 0 {
            let players_in_combat = players
                .iter_mut()
                .filter(|(_, _, in_combat)| in_combat.target == entity);

            for (player, _, _) in players_in_combat {
                bevy.entity(player).remove::<InCombat>();
            }

            let players_on_tile = siblings
                .map(|siblings| {
                    siblings
                        .iter()
                        .filter_map(|entity| players.get(*entity).ok())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            for (_, client, _) in players_on_tile {
                outbox.send_text(client.id, format!("{} has died.", depiction.name));
            }

            bevy.entity(entity).despawn();
        }
    }
}

pub fn on_player_death(
    mut bevy: Commands,
    mut npcs: Query<(Entity, &InCombat), With<Npc>>,
    mut outbox: EventWriter<Outbox>,
    mut proxy: EventWriter<ProxyCommand>,
    mut players: Query<(Entity, &Client, &mut State, &Attributes), (With<Online>, With<InCombat>)>,
    spawn_tiles: Query<Entity, With<DeathSpawn>>,
) {
    for (player, client, mut state, attributes) in players.iter_mut() {
        if state.health == 0 {
            outbox.send_text(client.id, "You have died.");

            bevy.entity(player).remove::<InCombat>();
            bevy.entity(player).remove::<QueuedAttack>();

            let npcs_in_combat = npcs
                .iter_mut()
                .filter(|(_, in_combat)| in_combat.target == player);

            for (npc, _) in npcs_in_combat {
                bevy.entity(npc).remove::<InCombat>();
            }

            if let Some(tile) = spawn_tiles.iter().next() {
                bevy.entity(player).set_parent(tile);
                state.health = attributes.max_health();

                proxy.send(ProxyCommand(ParsedCommand {
                    from: client.id,
                    command: Command::Look(None),
                }));
            }
        }
    }
}
