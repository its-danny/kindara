use std::sync::OnceLock;

use anyhow::Context;
use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    combat::{
        components::{Approach, AttackTimer, CombatState, Cooldowns, QueuedAttack, Stats},
        events::{CombatEvent, CombatEventKind, CombatEventTrigger},
    },
    data::resources::{Masteries, Skill, Skills},
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    npc::components::Npc,
    player::components::{Character, Client, Online},
    spatial::components::Tile,
    visual::components::Depiction,
};

static REGEX: OnceLock<Regex> = OnceLock::new();

pub fn handle_use_skill(content: &str) -> Result<Command, ParseError> {
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

            Ok(Command::UseSkill((skill, target)))
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct NpcQuery {
    entity: Entity,
    depiction: &'static Depiction,
    interactions: Option<&'static Interactions>,
    stats: Option<&'static Stats>,
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
    combat_state: Option<&'static CombatState>,
    attack_timer: Option<&'static AttackTimer>,
    queued_attack: Option<&'static mut QueuedAttack>,
    cooldowns: &'static Cooldowns,
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
pub fn use_skill(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    npcs: Query<NpcQuery>,
    mut players: Query<PlayerQuery>,
    mut outbox: EventWriter<Outbox>,
    mut combat_events: EventWriter<CombatEvent>,
    skills: Res<Skills>,
    masteries: Res<Masteries>,
    tiles: Query<TileQuery>,
) -> Result<(), anyhow::Error> {
    for command in commands.iter() {
        if let Command::UseSkill((skill, target)) = &command.command {
            let mut put_in_combat = false;

            let player = players
                .iter_mut()
                .find(|p| p.client.id == command.from)
                .context("Player not found")?;

            let skill = match get_skill(&skills, &masteries, &player.character, skill) {
                Ok(skill) => skill,
                Err(err) => {
                    outbox.send_text(player.client.id, err.to_string());

                    continue;
                }
            };

            if player.stats.status.vigor < skill.cost {
                outbox.send_text(
                    player.client.id,
                    format!("You don't have enough vigor to use {}.", skill.name),
                );

                continue;
            }

            if let Some(timer) = player.cooldowns.0.get(&skill.id) {
                outbox.send_text(
                    player.client.id,
                    format!(
                        "{} is on cooldown for {} seconds.",
                        skill.name,
                        timer.1.remaining().as_secs()
                    ),
                );

                continue;
            }

            if player.attack_timer.is_some() {
                match player.queued_attack {
                    Some(mut queued_attack) => {
                        queued_attack.0 = command.clone();

                        outbox.send_text(player.client.id, "Queued attack replaced.");
                    }
                    None => {
                        bevy.entity(player.entity)
                            .insert(QueuedAttack(command.clone()));

                        outbox.send_text(player.client.id, "Attack queued.");
                    }
                };
            }

            if let Some(target) = target {
                let target = match get_target(target, &tiles, &player.tile.get(), &npcs) {
                    Ok(entity) => entity,
                    Err(err) => {
                        outbox.send_text(player.client.id, err.to_string());

                        continue;
                    }
                };

                bevy.entity(player.entity).insert(CombatState {
                    target,
                    distance: skill.distance,
                    approach: Approach::Front,
                });

                bevy.entity(target).insert(CombatState {
                    target: player.entity,
                    distance: skill.distance,
                    approach: Approach::Front,
                });

                put_in_combat = true;
            }

            if player.combat_state.is_none() && !put_in_combat {
                outbox.send_text(player.client.id, "You are not in combat.");

                continue;
            }

            combat_events.send(CombatEvent {
                source: player.entity,
                trigger: CombatEventTrigger::Skill(skill.clone()),
                kind: CombatEventKind::Attack,
            })
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

#[cfg(test)]
mod tests {
    use crate::test::{
        app_builder::AppBuilder,
        npc_builder::NpcBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
        utils::send_message,
    };

    use super::*;

    use bevy::ecs::system::SystemState;
    use rstest::*;

    #[rstest]
    #[case("fireball goat", Some(("fireball".into(), Some("goat".into()))))]
    #[case("fireball", Some(("fireball".into(), None)))]
    fn parses(#[case] input: &str, #[case] expected: Option<(String, Option<String>)>) {
        let result = handle_use_skill(input)
            .ok()
            .and_then(|command| match command {
                Command::UseSkill((skill, target)) => Some((skill, target)),
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
    fn attacks() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, use_skill);

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

        assert_eq!(app.world.get::<CombatState>(player).unwrap().target, npc);
        assert_eq!(app.world.get::<CombatState>(npc).unwrap().target, player);
    }
}
