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

pub fn attack(
    mut bevy: Commands,
    mut commands: EventReader<ParsedCommand>,
    mut npcs: Query<NpcQuery>,
    mut players: Query<PlayerQuery>,
    mut outbox: EventWriter<Outbox>,
    mut prompts: EventWriter<Prompt>,
    skills: Res<Skills>,
    masteries: Res<Masteries>,
    tiles: Query<&Children, With<Tile>>,
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
                let siblings = value_or_continue!(tiles.get(player.tile.get()).ok());

                let target = match get_target(target, siblings, &npcs) {
                    Ok(entity) => entity,
                    Err(error) => {
                        outbox.send_text(player.client.id, error.to_string());

                        continue;
                    }
                };

                instantiate_combat(&mut bevy, target, &player, skill);
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

#[derive(Error, Debug)]
enum SkillError {
    #[error("You don't know how to do that.")]
    UnknownSkill,
}

fn get_skill<'a>(
    skills: &'a Res<'a, Skills>,
    masteries: &Res<Masteries>,
    character: &Character,
    skill: &String,
) -> Result<&'a Skill, SkillError> {
    masteries
        .0
        .get(&character.mastery)
        .filter(|mastery| mastery.skills.contains(skill))
        .and_then(|_| skills.0.get(skill))
        .ok_or(SkillError::UnknownSkill)
}

#[derive(Error, Debug)]
enum TargetError {
    #[error("You don't see a {0} here.")]
    NoTarget(String),
    #[error("You can't attack the {0}.")]
    InvalidTarget(String),
}

fn get_target(
    target: &String,
    siblings: &Children,
    npcs: &Query<NpcQuery>,
) -> Result<Entity, TargetError> {
    let npc = siblings
        .iter()
        .filter_map(|sibling| npcs.get(*sibling).ok())
        .find(|npc| npc.depiction.matches_query(&npc.entity, target))
        .ok_or_else(|| TargetError::NoTarget(target.clone()))?;

    if !npc
        .interactions
        .map_or(false, |i| i.0.contains(&Interaction::Attack))
    {
        return Err(TargetError::InvalidTarget(target.clone()));
    }

    if npc.state.is_none() {
        debug!("Target has Attack interaction but no state: {:?}", target);

        return Err(TargetError::InvalidTarget(target.clone()));
    }

    Ok(npc.entity)
}

fn instantiate_combat(
    bevy: &mut Commands,
    entity: Entity,
    player: &PlayerQueryReadOnlyItem,
    skill: &Skill,
) {
    bevy.entity(player.entity).insert(InCombat {
        target: entity,
        distance: skill.distance,
    });

    bevy.entity(entity).insert(InCombat {
        target: player.entity,
        distance: skill.distance,
    });
}

#[derive(Error, Debug)]
enum AttackError {
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

        return Err(AttackError::NotInCombat);
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
    use crate::test::{
        app_builder::AppBuilder,
        npc_builder::NpcBuilder,
        player_builder::PlayerBuilder,
        tile_builder::{TileBuilder, ZoneBuilder},
        utils::{get_message_content, send_message},
    };

    use super::*;

    #[test]
    fn parses() {
        let target = handle_attack("fireball goat");
        assert_eq!(
            target,
            Ok(Command::Attack(("fireball".into(), Some("goat".into()))))
        );

        let no_target = handle_attack("fireball");
        assert_eq!(no_target, Ok(Command::Attack(("fireball".into(), None))));
    }

    #[test]
    fn valid_target() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        let npc = NpcBuilder::new()
            .name("Pazuzu")
            .interactions(vec![Interaction::Attack])
            .tile(tile)
            .combat(true)
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch pazuzu");
        app.update();

        assert!(app.world.get::<InCombat>(player).is_some());
        assert!(app.world.get::<InCombat>(npc).is_some());
    }

    #[test]
    fn existing_target() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        NpcBuilder::new()
            .name("Pazuzu")
            .interactions(vec![Interaction::Attack])
            .tile(tile)
            .combat(true)
            .build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch pazuzu");
        app.update();

        send_message(&mut app, client_id, "punch");
        app.update();

        assert!(app.world.get::<InCombat>(player).is_some());
    }

    #[test]
    fn invalid_target() {
        let mut app = AppBuilder::new().build();
        app.add_systems(Update, attack);

        let zone = ZoneBuilder::new().build(&mut app);
        let tile = TileBuilder::new().build(&mut app, zone);

        NpcBuilder::new().name("Pazuzu").tile(tile).build(&mut app);

        let (player, client_id, _) = PlayerBuilder::new().tile(tile).build(&mut app);

        send_message(&mut app, client_id, "punch pazuzu");
        app.update();

        let content = get_message_content(&mut app, client_id).unwrap();

        assert_eq!(content, "You can't attack the pazuzu.");
        assert!(app.world.get::<InCombat>(player).is_none());
    }
}
