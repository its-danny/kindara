mod auth;
mod db;
mod player;
mod social;
mod spatial;
mod world;

use std::{env, time::Duration};

use bevy::{app::ScheduleRunnerSettings, asset::AssetPlugin, log::LogPlugin, prelude::*};
use bevy_nest::prelude::*;
use bevy_proto::prelude::*;
use dotenvy::dotenv;
use sqlx::{migrate, postgres::PgPoolOptions};

use crate::{
    auth::plugin::AuthPlugin, db::pool::DatabasePool, social::plugin::SocialPlugin,
    spatial::plugin::SpatialPlugin, world::plugin::WorldPlugin,
};

fn load_prototypes(mut prototypes: PrototypesMut) {
    prototypes.load_folder("world/").unwrap();
}

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
        // Resources
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .insert_resource(DatabasePool(pool))
        // Bevy plugins
        .add_plugins(MinimalPlugins)
        .add_plugin(AssetPlugin::default())
        .add_plugin(LogPlugin::default())
        // Prototypes
        .add_plugin(ProtoPlugin::new())
        .register_type::<spatial::components::Position>()
        .register_type::<spatial::components::Zone>()
        .register_type::<spatial::components::Tile>()
        // Our plugins
        .add_plugin(NestPlugin)
        .add_plugin(WorldPlugin)
        .add_plugin(AuthPlugin)
        .add_plugin(SpatialPlugin)
        .add_plugin(SocialPlugin)
        // Get it started
        .add_startup_systems((load_prototypes, setup_network))
        .run();

    Ok(())
}
