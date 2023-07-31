use bevy::prelude::*;
use bevy_nest::prelude::*;
use sqlx::PgPool;

use crate::{
    db::pool::DatabasePool,
    input::{
        events::{ParsedCommand, ProxyCommand},
        systems::{handle_proxy_command, parse_command},
    },
    player::events::Prompt,
    skills::resources::{Action, Skill, Skills},
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
            "attack".into(),
            Skill {
                name: "attack".into(),
                actions: vec![Action::ApplyDamage(10)],
            },
        );

        let mut app = App::new();

        app.add_plugins((MinimalPlugins, NestPlugin))
            .insert_resource(WorldState::default())
            .insert_resource(WorldTime::default())
            .insert_resource(skills)
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
