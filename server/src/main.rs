use std::time::Duration;

use bevy::{app::ScheduleRunnerSettings, log::LogPlugin, prelude::*};
use bevy_nest::prelude::*;

fn setup_network(server: Res<Server>) {
    server.listen("127.0.0.1:3000");
}

fn main() {
    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_plugins(MinimalPlugins)
        .add_plugin(LogPlugin {
            ..Default::default()
        })
        .add_plugin(NestPlugin)
        .add_startup_system(setup_network)
        .run();
}
