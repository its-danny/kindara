use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{
    input::events::{Command, ParsedCommand, ProxyCommand},
    npc::components::Npc,
    player::components::{Client, Online},
    spatial::components::{DeathSpawn, Tile},
    visual::components::Depiction,
};

use super::components::{HasAttacked, InCombat, QueuedAttack, Stats};

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
    npcs: Query<(Entity, &Depiction, &Stats, &Parent), With<Npc>>,
    mut players: Query<(Entity, &Client, &InCombat), With<Online>>,
    tiles: Query<&Children, With<Tile>>,
) {
    for (entity, depiction, stats, parent) in npcs.iter() {
        let siblings = tiles.get(parent.get()).ok();

        if stats.health == 0 {
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
    mut players: Query<(Entity, &Client, &mut Stats), (With<Online>, With<InCombat>)>,
    spawn_tiles: Query<Entity, With<DeathSpawn>>,
) {
    for (player, client, mut stats) in players.iter_mut() {
        if stats.health == 0 {
            outbox.send_text(client.id, "You have died.");

            bevy.entity(player).remove::<InCombat>();
            bevy.entity(player).remove::<QueuedAttack>();

            stats.health = stats.max_health();

            let npcs_in_combat = npcs
                .iter_mut()
                .filter(|(_, in_combat)| in_combat.target == player);

            for (npc, _) in npcs_in_combat {
                bevy.entity(npc).remove::<InCombat>();
            }

            if let Some(tile) = spawn_tiles.iter().next() {
                bevy.entity(player).set_parent(tile);

                proxy.send(ProxyCommand(ParsedCommand {
                    from: client.id,
                    command: Command::Look(None),
                }));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::*;
    use std::time::Duration;

    use crate::test::tile_builder::{TileBuilder, ZoneBuilder};
    use crate::test::utils::get_message_content;
    use crate::test::{
        app_builder::AppBuilder, npc_builder::NpcBuilder, player_builder::PlayerBuilder,
    };

    use crate::combat::components::Distance;

    use super::*;

    #[fixture]
    fn setup() -> (App, Entity, ClientId, Entity) {
        let mut app = AppBuilder::new().build();

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        let npc = NpcBuilder::new()
            .name("Goat")
            .combat(true)
            .tile(tile)
            .build(&mut app);

        app.world.entity_mut(player).insert(InCombat {
            target: npc,
            distance: Distance::Near,
        });

        (app, player, client_id, npc)
    }

    #[rstest]
    fn update_attack_timer_removes_component(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, player, _, _) = setup;
        app.add_systems(Update, update_attack_timer);

        let mut timer = Timer::from_seconds(1.0, TimerMode::Once);
        timer.set_elapsed(Duration::from_secs(1));
        app.world.entity_mut(player).insert(HasAttacked { timer });

        app.update();

        assert!(app.world.get::<HasAttacked>(player).is_none());
    }

    #[rstest]
    fn on_npc_death_destroys_entity(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, _, _, npc) = setup;
        app.add_systems(Update, on_npc_death);

        app.world.entity_mut(npc).insert(Stats::default());
        app.update();

        assert!(app.world.get_entity(npc).is_none());
    }

    #[rstest]
    fn on_npc_death_alerts_neighbors(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, _, client_id, npc) = setup;
        app.add_systems(Update, on_npc_death);

        app.world.entity_mut(npc).insert(Stats::default());

        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "Goat has died.");
    }

    #[rstest]
    fn on_player_death_resets_state(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, player, _, _) = setup;
        app.add_systems(Update, on_player_death);

        app.world.entity_mut(player).insert(Stats::default());
        app.update();

        assert!(app.world.get::<InCombat>(player).is_none());

        assert_eq!(
            app.world.get::<Stats>(player).unwrap().health,
            app.world.get::<Stats>(player).unwrap().max_health()
        );
    }

    #[rstest]
    fn on_player_death_teleports_player(setup: (App, Entity, ClientId, Entity)) {
        let (mut app, player, _, _) = setup;
        app.add_systems(Update, on_player_death);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        app.world.entity_mut(tile).insert(DeathSpawn);

        app.world.entity_mut(player).insert(Stats::default());
        app.update();

        assert_eq!(app.world.get::<Parent>(player).unwrap().get(), tile);
    }
}
