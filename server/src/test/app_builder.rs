use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{
    input::{events::ParsedCommand, systems::parse_command},
    world::resources::TileMap,
    Set,
};

pub struct AppBuilder;

impl AppBuilder {
    pub fn new() -> App {
        let mut app = App::new();

        app.configure_set(Set::Input.before(CoreSet::Update))
            .insert_resource(TileMap::default())
            .add_event::<Inbox>()
            .add_event::<Outbox>()
            .add_event::<ParsedCommand>()
            .add_system(parse_command.in_base_set(Set::Input));

        app
    }
}
