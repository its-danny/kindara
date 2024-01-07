use anyhow::Context;
use bevy::prelude::*;
use bevy_mod_sysfail::sysfail;
use bevy_nest::prelude::*;

use crate::{
    combat::components::{BlockCooldown, CombatState, DodgeCooldown, Stats},
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
    players: Query<
        (
            &Client,
            &Stats,
            Option<&CombatState>,
            Option<&DodgeCooldown>,
            Option<&BlockCooldown>,
        ),
        (With<Online>, Without<Npc>),
    >,
    npcs: Query<(&Stats, &Depiction), With<Npc>>,
) -> Result<(), anyhow::Error> {
    for prompt in events.iter() {
        let (client, stats, combat_state, dodge_cooldown, block_cooldown) = players
            .iter()
            .find(|(c, _, _, _, _)| c.id == prompt.client_id)
            .context("Player not found")?;

        let mut parts: Vec<String> = vec![];

        parts.push(paint!(
            "[{}/<fg.red>{}</> {}/<fg.cyan>{}</>{}{}]",
            stats.status.health,
            stats.max_health(),
            stats.status.vigor,
            stats.max_vigor(),
            if dodge_cooldown.is_none() { " d" } else { "" },
            if block_cooldown.is_none() { " b" } else { "" },
        ));

        if let Some(combat) = combat_state {
            let (stats, depiction) = npcs.get(combat.target)?;

            parts.push(paint!(
                "{} ({}, {}) [{}/<fg.red>{}</>]",
                depiction.name,
                combat.distance,
                combat.approach,
                stats.status.health,
                stats.max_health(),
            ));
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
