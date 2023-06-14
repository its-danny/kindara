use bevy::prelude::*;
use bevy_nest::prelude::*;
use sqlx::PgPool;

use crate::{
    db::pool::DatabasePool,
    input::{
        events::{ParsedCommand, ProxyCommand},
        systems::parse_command,
    },
    world::resources::WorldState,
    Set,
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
        let mut app = App::new();

        app.configure_set(Set::Input.before(CoreSet::Update))
            .add_plugins(MinimalPlugins)
            .insert_resource(WorldState::default())
            .add_event::<Inbox>()
            .add_event::<Outbox>()
            .add_event::<ParsedCommand>()
            .add_event::<ProxyCommand>()
            .add_system(parse_command.in_base_set(Set::Input));

        if let Some(database) = self.database {
            app.insert_resource(DatabasePool(database));
        }

        app
    }
}
