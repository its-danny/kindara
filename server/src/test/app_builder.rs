use bevy::prelude::*;
use bevy_nest::prelude::*;
use sqlx::PgPool;

use crate::{
    combat::components::Distance,
    db::pool::DatabasePool,
    input::{
        events::{ParsedCommand, ProxyCommand},
        systems::{handle_proxy_command, parse_command},
    },
    mastery::resources::{Masteries, Mastery},
    player::events::Prompt,
    skills::resources::{Action, RelevantStat, Skill, Skills},
    visual::paint,
    world::resources::{WorldState, WorldTime},
};

pub struct AppBuilder {
    database: Option<PgPool>,
}

impl AppBuilder {
    pub fn new() -> Self {
        Self { database: None }
    }

    pub fn database(mut self, pool: &PgPool) -> Self {
        self.database = Some(pool.clone());
        self
    }

    pub fn build(self) -> App {
        paint::toggle(false);

        let mut skills = Skills::default();

        skills.0.insert(
            "punch".into(),
            Skill {
                name: "Punch".into(),
                commands: vec!["punch".into()],
                stat: RelevantStat::Strength,
                distance: Distance::Near,
                cost: 0,
                cooldown: 0,
                actions: vec![Action::ApplyDamage("2d10".into())],
            },
        );

        let mut masteries = Masteries::default();

        masteries.0.insert(
            "boxer".into(),
            Mastery {
                name: "Boxer".into(),
                vitality: 0,
                proficiency: 0,
                speed: 0,
                strength: 0,
                dexterity: 0,
                intelligence: 0,
                skills: vec!["punch".into()],
            },
        );

        let mut app = App::new();

        app.add_plugins((MinimalPlugins, NestPlugin))
            .insert_resource(WorldState::default())
            .insert_resource(WorldTime::default())
            .insert_resource(skills)
            .insert_resource(masteries)
            .add_event::<Inbox>()
            .add_event::<Outbox>()
            .add_event::<ParsedCommand>()
            .add_event::<ProxyCommand>()
            .add_event::<Prompt>()
            .add_systems(First, (parse_command, handle_proxy_command));

        if let Some(database) = self.database {
            app.insert_resource(DatabasePool(database));
        }

        app
    }
}
