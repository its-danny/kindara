mod auth;
mod db;
mod input;
mod net;
mod player;
mod social;
mod spatial;
mod test;
mod visual;
mod world;

use std::{env, time::Duration};

use bevy::{app::ScheduleRunnerSettings, asset::AssetPlugin, log::LogPlugin, prelude::*};
use bevy_nest::prelude::*;
use bevy_proto::prelude::*;
use dotenvy::dotenv;
use sqlx::{migrate, postgres::PgPoolOptions};

use crate::{
    auth::plugin::AuthPlugin, db::pool::DatabasePool, input::plugin::InputPlugin,
    net::plugin::NetPlugin, player::plugin::PlayerPlugin, social::plugin::SocialPlugin,
    spatial::plugin::SpatialPlugin, visual::plugin::VisualPlugin, world::plugin::WorldPlugin,
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
pub enum Set {
    Input,
}

fn load_prototypes(mut prototypes: PrototypesMut) {
    prototypes.load_folder("world/").unwrap();
}

fn setup_network(server: Res<Server>) {
    server.listen(format!("127.0.0.1:{}", &env::var("SERVER_PORT").unwrap()));
}

#[async_std::main]
async fn main() -> Result<(), sqlx::Error> {
    dotenv().ok();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&env::var("DATABASE_URL").unwrap())
        .await?;

    migrate!().run(&pool).await?;

    App::new()
        // Stages
        .configure_set(Set::Input.before(CoreSet::Update))
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
        // Our plugins
        .add_plugin(WorldPlugin)
        .add_plugin(NestPlugin)
        .add_plugin(NetPlugin)
        .add_plugin(AuthPlugin)
        .add_plugin(InputPlugin)
        .add_plugin(PlayerPlugin)
        .add_plugin(SpatialPlugin)
        .add_plugin(SocialPlugin)
        .add_plugin(VisualPlugin)
        // Get it started
        .add_startup_systems((load_prototypes, setup_network))
        .run();

    Ok(())
}
