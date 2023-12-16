use std::sync::OnceLock;

use bevy::{ecs::query::WorldQuery, prelude::*};
use bevy_nest::prelude::*;
use regex::Regex;
use thiserror::Error;

use crate::{
    combat::components::{Attributes, HasAttacked, HitError, InCombat, QueuedAttack, State},
    input::events::{Command, ParseError, ParsedCommand},
    interact::components::{Interaction, Interactions},
    mastery::resources::Masteries,
    npc::components::Npc,
    player::{
        components::{Character, Client, Online},
        events::Prompt,
    },
    skills::resources::{Skill, Skills},
    spatial::components::Tile,
    value_or_continue,
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
    state: Option<&'static mut State>,
    with_npc: With<Npc>,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct PlayerQuery {
    entity: Entity,
    client: &'static Client,
    character: &'static mut Character,
    attributes: &'static Attributes,
    tile: &'static Parent,
    in_combat: Option<&'static InCombat>,
    has_attacked: Option<&'static HasAttacked>,
    queued_attack: Option<&'static mut QueuedAttack>,
    with_online: With<Online>,
}

#[derive(WorldQuery)]
pub struct TileQuery {
    entity: Entity,
    children: &'static Children,
    with_tile: With<Tile>,
}

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
) {
    for command in commands.iter() {
        if let Command::Attack((skill, target)) = &command.command {
            let player = value_or_continue!(players.iter().find(|p| p.client.id == command.from));

            let skill = match get_skill(&skills, &masteries, player.character, skill) {
                Ok(skill) => skill,
                Err(e) => {
                    outbox.send_text(player.client.id, e.to_string());

                    continue;
                }
            };

            if let Some(target) = target {
                let target = match get_target(target, &tiles, &player.tile.get(), &npcs) {
                    Ok(entity) => entity,
                    Err(error) => {
                        outbox.send_text(player.client.id, error.to_string());

                        continue;
                    }
                };

                instantiate_combat(&mut bevy, &target, &player.entity, skill);
            }

            let id = player.client.id;

            match execute_attack(&mut bevy, command, &mut npcs, &mut players, skill) {
                Ok(message) => outbox.send_text(id, message),
                Err(error) => outbox.send_text(id, error.to_string()),
            }

            prompts.send(Prompt::new(id));
        }
    }
}

#[derive(Error, Debug, PartialEq)]
enum SkillError {
    #[error("You don't know how to do that.")]
    UnknownSkill,
}

fn get_skill<'a>(
    skills: &'a Skills,
    masteries: &Masteries,
    character: &Character,
    skill: &str,
) -> Result<&'a Skill, SkillError> {
    masteries
        .0
        .get(&character.mastery)
        .filter(|mastery| mastery.skills.contains(&skill.to_string()))
        .and_then(|_| skills.0.get(skill))
        .ok_or(SkillError::UnknownSkill)
}

#[derive(Error, Debug, PartialEq)]
enum TargetError {
    #[error("You don't see anything here.")]
    NoTile,
    #[error("You don't see a {0} here.")]
    NoTarget(String),
    #[error("You can't attack the {0}.")]
    InvalidTarget(String),
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
        .ok_or_else(|| TargetError::NoTarget(target.into()))?;

    if !npc
        .interactions
        .map_or(false, |i| i.0.contains(&Interaction::Attack))
    {
        return Err(TargetError::InvalidTarget(target.into()));
    }

    if npc.state.is_none() {
        debug!("Target has Attack interaction but no state: {:?}", target);

        return Err(TargetError::InvalidTarget(target.into()));
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
) -> Result<String, AttackError> {
    let Some(player) = players
        .iter_mut()
        .find(|player| player.client.id == command.from)
    else {
        debug!("Player not found: {:?}", command.from);

        return Err(AttackError::NoPlayer);
    };

    let Some(in_combat) = player.in_combat else {
        return Err(AttackError::NotInCombat);
    };

    let npc = match npcs.get_mut(in_combat.target) {
        Ok(npc) => npc,
        Err(_) => return Err(AttackError::TargetNotFound),
    };

    let Some(mut state) = npc.state else {
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

    match in_combat.attack(bevy, player.entity, skill, player.attributes, &mut state) {
        Ok(_) => Ok(format!(
            "You attack the {}. It's health is now {}.",
            npc.depiction.short_name, state.health
        )),
        Err(HitError::Missed) => Ok(format!(
            "You attack the {} but miss.",
            npc.depiction.short_name
        )),
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
            utils::send_message,
        },
    };

    use super::*;

    use bevy::ecs::system::SystemState;
    use rstest::*;

    #[fixture]
    fn app() -> App {
        AppBuilder::new().build()
    }

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

    #[rstest]
    #[case(None, "punch", Ok("Punch".into()))]
    #[case(None, "kick", Err(SkillError::UnknownSkill))]
    #[case(Some("unknown"), "punch", Err(SkillError::UnknownSkill))]
    fn gets_skill(
        mut app: App,
        #[case] mastery: Option<&str>,
        #[case] skill: &str,
        #[case] expected: Result<String, SkillError>,
    ) {
        let (player, _, _) = PlayerBuilder::new().build(&mut app);

        if let Some(mastery) = mastery {
            let mut character = app.world.get_mut::<Character>(player).unwrap();
            character.mastery = mastery.to_string();
        }

        let skills = app.world.get_resource::<Skills>().unwrap();
        let masteries = app.world.get_resource::<Masteries>().unwrap();
        let character = app.world.get::<Character>(player).unwrap();

        let result = get_skill(skills, masteries, &character, skill).map(|s| s.name.clone());

        assert_eq!(result, expected);
    }

    #[rstest]
    #[case("goat", true, Ok("Goat".into()))]
    #[case("goat", false, Err(TargetError::InvalidTarget("goat".into())))]
    #[case("horse", false, Err(TargetError::NoTarget("horse".into())))]
    fn gets_target(
        mut app: App,
        #[case] name: &str,
        #[case] valid_target: bool,
        #[case] expected: Result<String, TargetError>,
    ) {
        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        PlayerBuilder::new().tile(tile).build(&mut app);

        NpcBuilder::new()
            .name("Goat")
            .tile(tile)
            .combat(valid_target)
            .interactions(if valid_target {
                vec![Interaction::Attack]
            } else {
                vec![]
            })
            .build(&mut app);

        let mut system_state: SystemState<(Query<NpcQuery>, Query<TileQuery>)> =
            SystemState::new(&mut app.world);
        let (npc_query, tile_query) = system_state.get_mut(&mut app.world);

        let result = get_target(&name, &tile_query, &tile, &npc_query).map(|e| {
            let depiction = app.world.get::<Depiction>(e).unwrap();
            depiction.name.clone()
        });

        assert_eq!(result, expected);
    }

    #[rstest]
    fn instantiates_combat(mut app: App) {
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

    #[rstest]
    #[case(false, false, false, false, Err(AttackError::NotInCombat))]
    #[case(true, false, false, false, Err(AttackError::InvalidTarget("Goat".into())))]
    #[case(true, true, true, false, Ok("Attack queued.".into()))]
    #[case(true, true, true, true, Ok("Queued attack replaced.".into()))]
    #[case(true, true, false, false, Ok("You attack the goat.".into()))]
    fn executes_attack(
        mut app: App,
        #[case] in_combat: bool,
        #[case] valid_target: bool,
        #[case] has_attacked: bool,
        #[case] queued_attack: bool,
        #[case] expected: Result<String, AttackError>,
    ) {
        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);
        let (player, client, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        let goat = NpcBuilder::new()
            .name("Goat")
            .short_name("goat")
            .tile(tile)
            .combat(valid_target)
            .interactions(if valid_target {
                vec![Interaction::Attack]
            } else {
                vec![]
            })
            .build(&mut app);

        if in_combat {
            app.world.entity_mut(player).insert(InCombat {
                target: goat,
                distance: Distance::Near,
            });

            app.world.entity_mut(goat).insert(InCombat {
                target: player,
                distance: Distance::Near,
            });
        }

        if has_attacked {
            app.world.entity_mut(player).insert(HasAttacked {
                timer: Timer::from_seconds(1.0, TimerMode::Once),
            });
        }

        if queued_attack {
            app.world
                .entity_mut(player)
                .insert(QueuedAttack(ParsedCommand {
                    from: client,
                    command: Command::Attack(("punch".into(), None)),
                }));
        }

        let mut system_state: SystemState<(
            Commands,
            Query<NpcQuery>,
            Query<PlayerQuery>,
            Res<Skills>,
        )> = SystemState::new(&mut app.world);
        let (mut commands, mut npc_query, mut player_query, skills) =
            system_state.get_mut(&mut app.world);

        let skill = skills.0.get("punch").unwrap();

        let result = execute_attack(
            &mut commands,
            &ParsedCommand {
                from: client,
                command: Command::Attack(("punch".into(), None)),
            },
            &mut npc_query,
            &mut player_query,
            &skill,
        )
        .map(|s| {
            if s.starts_with("You attack the goat") {
                "You attack the goat.".into()
            } else {
                s
            }
        });

        assert_eq!(result, expected);
    }

    #[test]
    fn attacks() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let npc = NpcBuilder::new()
            .name("Pazuzu")
            .tile(tile)
            .combat(true)
            .interactions(vec![Interaction::Attack])
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch pazuzu");
        app.update();

        assert!(app.world.get::<InCombat>(player).is_some());
        assert!(app.world.get::<InCombat>(npc).is_some());
    }
}
