use bevy_proto::prelude::*;

pub(super) fn spawn_void(mut commands: ProtoCommands) {
    commands.spawn("world-void");
}

pub(super) fn spawn_testing_movement(mut commands: ProtoCommands) {
    commands.spawn("world-testing-movement");
}
