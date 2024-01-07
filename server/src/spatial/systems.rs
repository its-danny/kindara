use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;

use crate::{
    input::events::{Command, ParsedCommand, ProxyCommand},
    player::components::Client,
};

use super::events::{MovementEvent, MovementEventKind};

#[sysfail(log)]
pub fn on_movement_event_flee(
    mut events: EventReader<MovementEvent>,
    mut proxy: EventWriter<ProxyCommand>,
    clients: Query<&Client>,
) -> Result<(), anyhow::Error> {
    for event in events.iter() {
        match &event.kind {
            MovementEventKind::Flee(direction) => {
                let client = clients.get(event.source).context("Client not found")?;

                proxy.send(ProxyCommand(ParsedCommand {
                    from: client.id,
                    command: Command::Movement((direction.clone(), true)),
                }));
            }
        }
    }

    Ok(())
}
