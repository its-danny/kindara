mod auth;
mod db;
mod player;
mod social;

use std::{env, time::Duration};

use bevy::{app::ScheduleRunnerSettings, log::LogPlugin, prelude::*};
use bevy_nest::prelude::*;
use dotenvy::dotenv;
use sqlx::{migrate, postgres::PgPoolOptions};

use crate::{auth::plugin::AuthPlugin, db::pool::DatabasePool, social::plugin::SocialPlugin};

fn setup_network(server: Res<Server>) {
    server.listen("127.0.0.1:3000");
}

#[async_std::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenv().ok();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env::var("DATABASE_URL").unwrap())
        .await?;

    migrate!("../migrations").run(&pool).await?;

    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(DatabasePool(pool))
        .add_plugins(MinimalPlugins)
        .add_plugin(LogPlugin {
            ..Default::default()
        })
        .add_plugin(NestPlugin)
        .add_plugin(AuthPlugin)
        .add_plugin(SocialPlugin)
        .add_startup_system(setup_network)
        .run();

    Ok(())
}
