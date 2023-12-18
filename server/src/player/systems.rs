use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;

use crate::{
    combat::components::{Attributes, InCombat, State},
    net::telnet::NAWS,
    npc::components::Npc,
    paint,
    visual::components::Depiction,
};

use super::{
    components::{Client, Online},
    events::Prompt,
    resources::PromptTimer,
};

#[sysfail(log)]
pub fn handle_client_width(
    mut inbox: EventReader<Inbox>,
    mut clients: Query<&mut Client>,
) -> Result<(), anyhow::Error> {
    for (message, content) in inbox.iter().filter_map(|m| {
        if let Message::Command(content) = &m.content {
            Some((m, content))
        } else {
            None
        }
    }) {
        let mut client = clients
            .iter_mut()
            .find(|c| c.id == message.from)
            .context("Client not found")?;

        if content[0..=2] == [IAC, SB, NAWS] {
            let width = content.get(4).map(|a| *a as u16).unwrap_or(80);

            if width > 0 {
                client.width = width;
            }
        }
    }

    Ok(())
}

#[sysfail(log)]
pub fn send_prompt(
    mut events: EventReader<Prompt>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Attributes, &State, Option<&InCombat>)>,
    npcs: Query<&Depiction, With<Npc>>,
) -> Result<(), anyhow::Error> {
    for prompt in events.iter() {
        let (client, attributes, state, in_combat) = players
            .iter()
            .find(|(c, _, _, _)| c.id == prompt.client_id)
            .context("Player not found")?;

        let mut parts: Vec<String> = vec![];

        parts.push(paint!(
            "[{}/<fg.red>{}</>]",
            state.health,
            attributes.max_health()
        ));

        if let Some(combat) = in_combat {
            let depiction = npcs.get(combat.target)?;

            parts.push(format!("{} ({})", depiction.name, combat.distance));
        }

        parts.push("->".into());

        outbox.send_text(client.id, parts.join(" "));
    }

    Ok(())
}

pub fn send_prompt_on_timer(
    mut prompts: EventWriter<Prompt>,
    mut timer: ResMut<PromptTimer>,
    players: Query<&Client, With<Online>>,
    time: Res<Time>,
) {
    if timer.0.tick(time.delta()).just_finished() {
        for client in players.iter() {
            prompts.send(Prompt::new(client.id));
        }
    }
}
