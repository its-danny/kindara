use bevy_proto::prelude::*;

pub fn spawn_trinus_castra(mut commands: ProtoCommands) {
    commands.spawn("world.trinus.trinus-castra");
}

pub fn spawn_the_roaring_lion(mut commands: ProtoCommands) {
    commands.spawn("world.trinus.the-roaring-lion");
}
