use bevy::prelude::*;
use bevy_nest::prelude::*;
use sqlx::PgPool;

use crate::{
    combat::{components::Distance, events::CombatEvent},
    data::resources::{Masteries, Mastery},
    data::resources::{Skill, Skills},
    db::pool::DatabasePool,
    input::{
        events::{ParsedCommand, ProxyCommand},
        systems::{handle_proxy_command, parse_command},
    },
    player::events::Prompt,
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
                id: "punch".into(),
                commands: vec!["punch".into()],
                name: "Punch".into(),
                description: "You sock 'em in the jaw.".into(),
                cost: 0,
                cooldown: 0,
                distance: Distance::Near,
                dodge_difficulty: 0.0,
                block_difficulty: 0.0,
                scripts: vec![],
            },
        );

        let mut masteries = Masteries::default();

        masteries.0.insert(
            "freelancer".into(),
            Mastery {
                id: "freelancer".into(),
                name: "Freelancer".into(),
                vitality: 0,
                stamina: 0,
                strength: 0,
                dexterity: 0,
                intelligence: 0,
                auto_attack: "punch".into(),
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
            .add_event::<CombatEvent>()
            .add_systems(First, (parse_command, handle_proxy_command));

        if let Some(database) = self.database {
            app.insert_resource(DatabasePool(database));
        }

        app
    }
}
