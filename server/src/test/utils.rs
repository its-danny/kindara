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
    let event = outbox_reader
        .iter(outbox_events)
        .find(|r| r.to == to)
        .expect("Expected response");

    match &event.content {
        Message::Text(text) => text.clone(),
        _ => panic!("Expected Message::Text"),
    }
}

pub fn get_task<T: Component>(app: &mut App) -> Option<&T> {
    app.world.query::<&mut T>().iter(&app.world).next()
}

pub fn wait_for_task<T>(task: &Task<T>) {
    while !task.is_finished() {
        tick_global_task_pools_on_main_thread();
    }
}
