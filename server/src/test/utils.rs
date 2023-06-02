use bevy::prelude::*;
use bevy::tasks::*;
use bevy_nest::prelude::*;

pub fn send_message(app: &mut App, from: ClientId, message: &str) {
    app.world.resource_mut::<Events<Inbox>>().send(Inbox {
        from,
        content: Message::Text(message.into()),
    });
}

pub fn get_message_content(app: &mut App, to: ClientId) -> String {
    let outbox_events = app.world.resource::<Events<Outbox>>();
    let mut outbox_reader = outbox_events.get_reader();

    outbox_reader
        .iter(outbox_events)
        .filter(|e| e.to == to)
        .find_map(|e| match &e.content {
            Message::Text(text) => Some(text.clone()),
            _ => None,
        })
        .expect("Expected Message::Text")
}

pub fn get_command_content(app: &mut App, to: ClientId) -> Vec<u8> {
    let outbox_events = app.world.resource::<Events<Outbox>>();
    let mut outbox_reader = outbox_events.get_reader();

    outbox_reader
        .iter(outbox_events)
        .filter(|e| e.to == to)
        .find_map(|e| match &e.content {
            Message::Command(command) => Some(command.clone()),
            _ => None,
        })
        .expect("Expected Message::Command")
}

pub fn get_task<T: Component>(app: &mut App) -> Option<&T> {
    app.world.query::<&mut T>().iter(&app.world).next()
}

pub fn wait_for_task<T>(task: &Task<T>) {
    while !task.is_finished() {
        tick_global_task_pools_on_main_thread();
    }
}
