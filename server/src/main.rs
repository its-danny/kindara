mod auth;
mod combat;
mod data;
mod db;
mod input;
mod interact;
mod items;
mod keycard;
mod lua;
mod menu;
mod net;
mod npc;
mod player;
mod social;
mod spatial;
mod test;
mod values;
mod visual;
mod world;

use std::{env, time::Duration};

use bevy::{
    app::ScheduleRunnerPlugin,
    asset::AssetPlugin,
    log::{Level, LogPlugin},
    prelude::*,
    time::TimePlugin,
};
use bevy_nest::prelude::*;
use bevy_proto::prelude::*;
use dotenvy::dotenv;
use sqlx::{migrate, postgres::PgPoolOptions};

use crate::{
    auth::plugin::AuthPlugin, combat::plugin::CombatPlugin, data::plugin::DataPlugin,
    db::pool::DatabasePool, input::plugin::InputPlugin, interact::plugin::InteractPlugin,
    items::plugin::ItemPlugin, lua::plugin::LuaPlugin, menu::plugin::MenuPlugin,
    net::plugin::NetPlugin, npc::plugin::NpcPlugin, player::plugin::PlayerPlugin,
    social::plugin::SocialPlugin, spatial::plugin::SpatialPlugin, visual::plugin::VisualPlugin,
    world::plugin::WorldPlugin,
};

fn load_prototypes(mut prototypes: PrototypesMut) {
    match prototypes.load_folder("prototypes/world/") {
        Ok(loaded) => {
            loaded.iter().for_each(|proto| {
                info!("Loaded zones: {:?}", proto);
            });
        }
        Err(err) => {
            error!("Failed to load prototypes: {}", err);
        }
    }

    match prototypes.load_folder("prototypes/hostiles/") {
        Ok(loaded) => {
            loaded.iter().for_each(|proto| {
                info!("Loaded hostiles: {:?}", proto);
            });
        }
        Err(err) => {
            error!("Failed to load prototypes: {}", err);
        }
    }
}

fn setup_network(server: Res<Server>) {
    server.listen(format!("0.0.0.0:{}", &env::var("SERVER_PORT").unwrap()));
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
        // Resources
        .insert_resource(DatabasePool(pool))
        // Bevy plugins
        .add_plugins((
            AssetPlugin::default(),
            LogPlugin {
                level: Level::DEBUG,
                ..Default::default()
            },
            TaskPoolPlugin::default(),
            TypeRegistrationPlugin,
            TimePlugin,
            ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0)),
        ))
        // 3rd party plugins
        .add_plugins((NestPlugin, ProtoPlugin::new()))
        // Our plugins
        .add_plugins((
            AuthPlugin,
            CombatPlugin,
            DataPlugin,
            InputPlugin,
            InteractPlugin,
            ItemPlugin,
            LuaPlugin,
            MenuPlugin,
            NetPlugin,
            NpcPlugin,
            PlayerPlugin,
            SocialPlugin,
            SpatialPlugin,
            VisualPlugin,
            WorldPlugin,
        ))
        // Get it started
        .add_systems(Startup, (load_prototypes, setup_network))
        .run();

    Ok(())
}
