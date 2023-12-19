use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    combat::components::{HasAttacked, HitError, InCombat, QueuedAttack, Stats},
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    mastery::resources::Masteries,
    npc::components::Npc,
    player::{
        components::{Character, Client, Online},
        events::Prompt,
    },
    skills::{
        components::Cooldowns,
        resources::{Skill, Skills},
    },
    spatial::components::Tile,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_attack(content: &str) -> Result<Command, ParseError> {
    let regex = REGEX.get_or_init(|| Regex::new(r"^(?P<skill>.*?)( (?P<target>.*?))?$").unwrap());

    match regex.captures(content) {
        None => Err(ParseError::WrongCommand),
        Some(captures) => {
            let skill = captures
                .name("skill")
                .map(|m| m.as_str().trim().to_lowercase())
                .ok_or(ParseError::InvalidArguments("What skill?".into()))?;

            let target = captures
                .name("target")
                .map(|m| m.as_str().trim().to_lowercase());

            Ok(Command::Attack((skill, target)))
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct NpcQuery {
    entity: Entity,
    depiction: &'static Depiction,
    interactions: Option<&'static Interactions>,
    stats: Option<&'static mut Stats>,
    with_npc: With<Npc>,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct PlayerQuery {
    entity: Entity,
    client: &'static Client,
    character: &'static mut Character,
    stats: &'static mut Stats,
    tile: &'static Parent,
    in_combat: Option<&'static InCombat>,
    has_attacked: Option<&'static HasAttacked>,
    queued_attack: Option<&'static mut QueuedAttack>,
    cooldowns: &'static mut Cooldowns,
    with_online: With<Online>,
    without_npc: Without<Npc>,
}

#[derive(WorldQuery)]
pub struct TileQuery {
    entity: Entity,
    children: &'static Children,
    with_tile: With<Tile>,
}

#[sysfail(log)]
pub fn attack(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut npcs: Query<NpcQuery>,
    mut players: Query<PlayerQuery>,
    mut outbox: EventWriter<Outbox>,
    mut prompts: EventWriter<Prompt>,
    skills: Res<Skills>,
    masteries: Res<Masteries>,
    tiles: Query<TileQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::Attack((skill, target)) = &command.command {
            let player = players
                .iter()
                .find(|p| p.client.id == command.from)
                .context("Player not found")?;

            let mut in_combat: Option<InCombat> = player.in_combat.cloned();

            let skill = match get_skill(&skills, &masteries, player.character, skill) {
                Ok(skill) => skill,
                Err(err) => {
                    outbox.send_text(player.client.id, err.to_string());

                    continue;
                }
            };

            if player.stats.potential < skill.cost {
                outbox.send_text(
                    player.client.id,
                    format!("You don't have enough potential to use {}.", skill.name),
                );

                continue;
            }

            if let Some(timer) = player.cooldowns.0.get(&skill.name) {
                outbox.send_text(
                    player.client.id,
                    format!(
                        "{} is on cooldown for {} more seconds.",
                        skill.name,
                        timer.remaining().as_secs()
                    ),
                );

                continue;
            }

            if let Some(target) = target {
                let target = match get_target(target, &tiles, &player.tile.get(), &npcs) {
                    Ok(entity) => entity,
                    Err(err) => {
                        outbox.send_text(player.client.id, err.to_string());

                        continue;
                    }
                };

                // We store this here because the insert in `instantiate_combat` won't
                // happen until next game tick. This lets the player attack immediately.
                in_combat = Some(InCombat {
                    target,
                    distance: skill.distance,
                });

                instantiate_combat(&mut bevy, &target, &player.entity, skill);
            }

            let id = player.client.id;

            match execute_attack(
                &mut bevy,
                command,
                &mut npcs,
                &mut players,
                skill,
                &in_combat.as_ref(),
            ) {
                Ok(message) => outbox.send_text(id, message),
                Err(err) => outbox.send_text(id, err.to_string()),
            }

            prompts.send(Prompt::new(id));
        }
    }

    Ok(())
}

#[derive(Error, Debug, PartialEq)]
enum SkillError {
    #[error("You don't know how to do that.")]
    Unknown,
}

fn get_skill<'a>(
    skills: &'a Skills,
    masteries: &Masteries,
    character: &Character,
    skill: &str,
) -> Result<&'a Skill, SkillError> {
    let (key, skill) = skills
        .0
        .iter()
        .find(|(_, s)| s.commands.contains(&skill.to_string()))
        .ok_or(SkillError::Unknown)?;

    let mastery = masteries
        .0
        .get(&character.mastery)
        .ok_or(SkillError::Unknown)?;

    if !mastery.skills.contains(key) {
        return Err(SkillError::Unknown);
    }

    Ok(skill)
}

#[derive(Error, Debug, PartialEq)]
enum TargetError {
    #[error("You don't see anything here.")]
    NoTile,
    #[error("You don't see a {0} here.")]
    NotFound(String),
    #[error("You can't attack the {0}.")]
    Invalid(String),
}

fn get_target(
    target: &str,
    tiles: &Query<TileQuery>,
    tile: &Entity,
    npcs: &Query<NpcQuery>,
) -> Result<Entity, TargetError> {
    let siblings = tiles.get(*tile).ok().ok_or(TargetError::NoTile)?;

    let npc = siblings
        .children
        .iter()
        .filter_map(|sibling| npcs.get(*sibling).ok())
        .find(|npc| npc.depiction.matches_query(&npc.entity, target))
        .ok_or_else(|| TargetError::NotFound(target.into()))?;

    if !npc
        .interactions
        .map_or(false, |i| i.0.contains(&Interaction::Attack))
    {
        return Err(TargetError::Invalid(target.into()));
    }

    if npc.stats.is_none() {
        debug!("Target has Attack interaction but no state: {:?}", target);

        return Err(TargetError::Invalid(target.into()));
    }

    Ok(npc.entity)
}

fn instantiate_combat(bevy: &mut Commands, target: &Entity, player: &Entity, skill: &Skill) {
    bevy.entity(*player).insert(InCombat {
        target: *target,
        distance: skill.distance,
    });

    bevy.entity(*target).insert(InCombat {
        target: *player,
        distance: skill.distance,
    });
}

#[derive(Error, Debug, PartialEq)]
enum AttackError {
    #[error("You don't see anything here.")]
    NoPlayer,
    #[error("You are not in combat.")]
    NotInCombat,
    #[error("You can't attack the {0}.")]
    InvalidTarget(String),
    #[error("Could not find target.")]
    TargetNotFound,
}

fn execute_attack(
    bevy: &mut Commands,
    command: &ParsedCommand,
    npcs: &mut Query<NpcQuery>,
    players: &mut Query<PlayerQuery>,
    skill: &Skill,
    in_combat: &Option<&InCombat>,
) -> Result<String, AttackError> {
    let Some(mut player) = players
        .iter_mut()
        .find(|player| player.client.id == command.from)
    else {
        debug!("Player not found: {:?}", command.from);

        return Err(AttackError::NoPlayer);
    };

    let Some(in_combat) = in_combat else {
        return Err(AttackError::NotInCombat);
    };

    let npc = match npcs.get_mut(in_combat.target) {
        Ok(npc) => npc,
        Err(_) => return Err(AttackError::TargetNotFound),
    };

    let Some(mut state) = npc.stats else {
        debug!(
            "Target has Attack interaction but no state: {:?}",
            npc.depiction.name
        );

        return Err(AttackError::InvalidTarget(npc.depiction.name.clone()));
    };

    if player.has_attacked.is_some() {
        return match player.queued_attack {
            Some(mut queued_attack) => {
                queued_attack.0 = command.clone();

                Ok("Queued attack replaced.".into())
            }
            None => {
                bevy.entity(player.entity)
                    .insert(QueuedAttack(command.clone()));

                Ok("Attack queued.".into())
            }
        };
    }

    player.stats.potential = player.stats.potential.saturating_sub(skill.cost);

    player.cooldowns.0.insert(
        skill.name.clone(),
        Timer::from_seconds(skill.cooldown as f32, TimerMode::Once),
    );

    match in_combat.attack(bevy, player.entity, skill, &player.stats, &mut state) {
        Ok(_) => Ok(skill.flavor.clone()),
        Err(HitError::Missed) => Ok("You miss.".into()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        combat::components::Distance,
        test::{
            app_builder::AppBuilder,
            npc_builder::NpcBuilder,
            player_builder::PlayerBuilder,
            tile_builder::{TileBuilder, ZoneBuilder},
            utils::{get_message_content, send_message},
        },
    };

    use super::*;

    use bevy::ecs::system::SystemState;
    use rstest::*;

    #[rstest]
    #[case("fireball goat", Some(("fireball".into(), Some("goat".into()))))]
    #[case("fireball", Some(("fireball".into(), None)))]
    fn parses(#[case] input: &str, #[case] expected: Option<(String, Option<String>)>) {
        let result = handle_attack(input).ok().and_then(|command| match command {
            Command::Attack((skill, target)) => Some((skill, target)),
            _ => None,
        });

        assert_eq!(result, expected);
    }

    #[fixture]
    fn get_skill_setup() -> (App, Entity, Entity) {
        let mut app = AppBuilder::new().build();

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);
        let player = PlayerBuilder::new().tile(tile).build(&mut app);

        (app, tile, player.0)
    }

    #[rstest]
    fn get_skill_valid(get_skill_setup: (App, Entity, Entity)) {
        let (mut app, _, player) = get_skill_setup;

        let mut system_state: SystemState<(Res<Skills>, Res<Masteries>, Query<&Character>)> =
            SystemState::new(&mut app.world);
        let (skills, masteries, character_query) = system_state.get(&mut app.world);
        let character = character_query.get(player).unwrap();

        let result = get_skill(
            skills.into_inner(),
            masteries.into_inner(),
            &character,
            "punch",
        )
        .map(|s| s.name.clone());

        assert_eq!(result, Ok("Punch".into()));
    }

    #[rstest]
    fn get_skill_invalid_mastery(get_skill_setup: (App, Entity, Entity)) {
        let (mut app, _, player) = get_skill_setup;

        let mut system_state: SystemState<(Res<Skills>, Res<Masteries>, Query<&mut Character>)> =
            SystemState::new(&mut app.world);
        let (skills, masteries, mut character_query) = system_state.get_mut(&mut app.world);
        let mut character = character_query.get_mut(player).unwrap();

        character.mastery = "unknown".into();

        let result = get_skill(
            skills.into_inner(),
            masteries.into_inner(),
            &character,
            "punch",
        )
        .map(|s| s.name.clone());

        assert_eq!(result, Err(SkillError::Unknown));
    }

    #[rstest]
    fn get_skill_invalid_skill(get_skill_setup: (App, Entity, Entity)) {
        let (mut app, _, player) = get_skill_setup;

        let mut system_state: SystemState<(Res<Skills>, Res<Masteries>, Query<&Character>)> =
            SystemState::new(&mut app.world);
        let (skills, masteries, character_query) = system_state.get(&mut app.world);
        let character = character_query.get(player).unwrap();

        let result = get_skill(
            skills.into_inner(),
            masteries.into_inner(),
            &character,
            "kick",
        )
        .map(|s| s.name.clone());

        assert_eq!(result, Err(SkillError::Unknown));
    }

    #[fixture]
    fn get_target_setup() -> (App, Entity, Entity) {
        let mut app = AppBuilder::new().build();

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);
        let player = PlayerBuilder::new().tile(tile).build(&mut app);

        (app, tile, player.0)
    }

    #[rstest]
    fn get_target_valid(get_target_setup: (App, Entity, Entity)) {
        let (mut app, tile, _) = get_target_setup;

        let goat = NpcBuilder::new()
            .name("Goat")
            .tile(tile)
            .combat(true)
            .build(&mut app);

        let mut system_state: SystemState<(Query<NpcQuery>, Query<TileQuery>)> =
            SystemState::new(&mut app.world);
        let (npc_query, tile_query) = system_state.get_mut(&mut app.world);

        let result = get_target("goat", &tile_query, &tile, &npc_query);

        assert_eq!(result, Ok(goat));
    }

    #[rstest]
    fn get_target_invalid(get_target_setup: (App, Entity, Entity)) {
        let (mut app, tile, _) = get_target_setup;

        NpcBuilder::new().name("Goat").tile(tile).build(&mut app);

        let mut system_state: SystemState<(Query<NpcQuery>, Query<TileQuery>)> =
            SystemState::new(&mut app.world);
        let (npc_query, tile_query) = system_state.get_mut(&mut app.world);

        let result = get_target("goat", &tile_query, &tile, &npc_query);

        assert_eq!(result, Err(TargetError::Invalid("goat".into())));
    }

    #[rstest]
    fn get_target_not_found(get_target_setup: (App, Entity, Entity)) {
        let (mut app, tile, _) = get_target_setup;

        let mut system_state: SystemState<(Query<NpcQuery>, Query<TileQuery>)> =
            SystemState::new(&mut app.world);
        let (npc_query, tile_query) = system_state.get_mut(&mut app.world);

        let result = get_target("goat", &tile_query, &tile, &npc_query);

        assert_eq!(result, Err(TargetError::NotFound("goat".into())));
    }

    #[test]
    fn instantiates_combat() {
        let mut app = AppBuilder::new().build();
        let npc = NpcBuilder::new().build(&mut app);
        let (player, _, _) = PlayerBuilder::new().build(&mut app);

        let mut system_state: SystemState<(Commands, Res<Skills>)> =
            SystemState::new(&mut app.world);
        let (mut commands, skills) = system_state.get_mut(&mut app.world);
        let skill = skills.0.get("punch").unwrap();

        instantiate_combat(&mut commands, &npc, &player, &skill);
        system_state.apply(&mut app.world);

        let player_in_combat = app.world.get::<InCombat>(player).unwrap();
        let npc_in_combat = app.world.get::<InCombat>(npc).unwrap();

        assert_eq!(player_in_combat.target, npc);
        assert_eq!(npc_in_combat.target, player);
    }

    struct ExecuteAttackSetup {
        app: App,
        tile: Entity,
        player: Entity,
        client: ClientId,
        command: ParsedCommand,
    }

    #[fixture]
    fn execute_attack_setup() -> ExecuteAttackSetup {
        let mut app = AppBuilder::new().build();
        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);
        let (player, client, _) = PlayerBuilder::new().tile(tile).build(&mut app);
        let command = ParsedCommand {
            from: client,
            command: Command::Attack(("punch".into(), None)),
        };

        ExecuteAttackSetup {
            app,
            tile,
            player,
            client,
            command,
        }
    }

    #[rstest]
    fn execute_attack_ready(execute_attack_setup: ExecuteAttackSetup) {
        let ExecuteAttackSetup {
            mut app,
            tile,
            player,
            command,
            ..
        } = execute_attack_setup;

        let goat = NpcBuilder::new()
            .name("Goat")
            .short_name("goat")
            .tile(tile)
            .combat(true)
            .build(&mut app);

        app.world.entity_mut(player).insert(InCombat {
            target: goat,
            distance: Distance::Near,
        });

        app.world.entity_mut(goat).insert(InCombat {
            target: player,
            distance: Distance::Near,
        });

        let mut system_state: SystemState<(
            Commands,
            Query<NpcQuery>,
            Query<PlayerQuery>,
            Res<Skills>,
        )> = SystemState::new(&mut app.world);
        let (mut commands, mut npc_query, mut player_query, skills) =
            system_state.get_mut(&mut app.world);
        let in_combat = player_query.get_mut(player).unwrap().in_combat.cloned();
        let skill = skills.0.get("punch").unwrap();

        let result = execute_attack(
            &mut commands,
            &command,
            &mut npc_query,
            &mut player_query,
            &skill,
            &in_combat.as_ref(),
        );

        assert_eq!(result, Ok("You sock 'em in the jaw.".into()));
    }

    #[rstest]
    fn execute_attack_queued(execute_attack_setup: ExecuteAttackSetup) {
        let ExecuteAttackSetup {
            mut app,
            tile,
            player,
            command,
            ..
        } = execute_attack_setup;

        let goat = NpcBuilder::new()
            .name("Goat")
            .short_name("goat")
            .tile(tile)
            .combat(true)
            .build(&mut app);

        app.world.entity_mut(player).insert(InCombat {
            target: goat,
            distance: Distance::Near,
        });

        app.world.entity_mut(goat).insert(InCombat {
            target: player,
            distance: Distance::Near,
        });

        app.world.entity_mut(player).insert(HasAttacked {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        });

        let mut system_state: SystemState<(
            Commands,
            Query<NpcQuery>,
            Query<PlayerQuery>,
            Res<Skills>,
        )> = SystemState::new(&mut app.world);
        let (mut commands, mut npc_query, mut player_query, skills) =
            system_state.get_mut(&mut app.world);
        let in_combat = player_query.get_mut(player).unwrap().in_combat.cloned();
        let skill = skills.0.get("punch").unwrap();

        let result = execute_attack(
            &mut commands,
            &command,
            &mut npc_query,
            &mut player_query,
            &skill,
            &in_combat.as_ref(),
        );

        assert_eq!(result, Ok("Attack queued.".into()));
    }

    #[rstest]
    fn execute_attack_queue_replaced(execute_attack_setup: ExecuteAttackSetup) {
        let ExecuteAttackSetup {
            mut app,
            tile,
            player,
            client,
            command,
        } = execute_attack_setup;

        let goat = NpcBuilder::new()
            .name("Goat")
            .short_name("goat")
            .tile(tile)
            .combat(true)
            .build(&mut app);

        app.world.entity_mut(player).insert(InCombat {
            target: goat,
            distance: Distance::Near,
        });

        app.world.entity_mut(goat).insert(InCombat {
            target: player,
            distance: Distance::Near,
        });

        app.world.entity_mut(player).insert(HasAttacked {
            timer: Timer::from_seconds(1.0, TimerMode::Once),
        });

        app.world
            .entity_mut(player)
            .insert(QueuedAttack(ParsedCommand {
                from: client,
                command: Command::Attack(("punch".into(), None)),
            }));

        let mut system_state: SystemState<(
            Commands,
            Query<NpcQuery>,
            Query<PlayerQuery>,
            Res<Skills>,
        )> = SystemState::new(&mut app.world);
        let (mut commands, mut npc_query, mut player_query, skills) =
            system_state.get_mut(&mut app.world);
        let in_combat = player_query.get_mut(player).unwrap().in_combat.cloned();
        let skill = skills.0.get("punch").unwrap();

        let result = execute_attack(
            &mut commands,
            &command,
            &mut npc_query,
            &mut player_query,
            &skill,
            &in_combat.as_ref(),
        );

        assert_eq!(result, Ok("Queued attack replaced.".into()));
    }

    #[rstest]
    fn execute_attack_invalid_target(execute_attack_setup: ExecuteAttackSetup) {
        let ExecuteAttackSetup {
            mut app,
            tile,
            player,
            command,
            ..
        } = execute_attack_setup;

        let goat = NpcBuilder::new()
            .name("Goat")
            .short_name("goat")
            .tile(tile)
            .combat(false)
            .build(&mut app);

        app.world.entity_mut(player).insert(InCombat {
            target: goat,
            distance: Distance::Near,
        });

        let mut system_state: SystemState<(
            Commands,
            Query<NpcQuery>,
            Query<PlayerQuery>,
            Res<Skills>,
        )> = SystemState::new(&mut app.world);
        let (mut commands, mut npc_query, mut player_query, skills) =
            system_state.get_mut(&mut app.world);
        let in_combat = player_query.get_mut(player).unwrap().in_combat.cloned();
        let skill = skills.0.get("punch").unwrap();

        let result = execute_attack(
            &mut commands,
            &command,
            &mut npc_query,
            &mut player_query,
            &skill,
            &in_combat.as_ref(),
        );

        assert_eq!(result, Err(AttackError::InvalidTarget("Goat".into())));
    }

    #[rstest]
    fn execute_attack_not_in_combat(execute_attack_setup: ExecuteAttackSetup) {
        let ExecuteAttackSetup {
            mut app,
            command,
            player,
            ..
        } = execute_attack_setup;

        let mut system_state: SystemState<(
            Commands,
            Query<NpcQuery>,
            Query<PlayerQuery>,
            Res<Skills>,
        )> = SystemState::new(&mut app.world);
        let (mut commands, mut npc_query, mut player_query, skills) =
            system_state.get_mut(&mut app.world);
        let in_combat = player_query.get(player).unwrap().in_combat.cloned();
        let skill = skills.0.get("punch").unwrap();

        let result = execute_attack(
            &mut commands,
            &command,
            &mut npc_query,
            &mut player_query,
            &skill,
            &in_combat.as_ref(),
        );

        assert_eq!(result, Err(AttackError::NotInCombat));
    }

    #[test]
    fn attacks() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let npc = NpcBuilder::new()
            .name("Goat")
            .short_name("goat")
            .tile(tile)
            .combat(true)
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch goat");
        app.update();

        assert_eq!(app.world.get::<InCombat>(player).unwrap().target, npc);
        assert_eq!(app.world.get::<InCombat>(npc).unwrap().target, player);

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You sock 'em in the jaw.");
    }
}
