use bevy::prelude::*;
use bevy_nest::prelude::*;

use crate::{
    combat::components::{Attributes, State},
    net::telnet::NAWS,
    npc::components::Npc,
    paint, value_or_continue,
    visual::components::Depiction,
};

use super::{
    components::{Character, CharacterState, Client, Online},
    events::Prompt,
    resources::PromptTimer,
};

pub fn handle_client_width(mut inbox: EventReader<Inbox>, mut clients: Query<&mut Client>) {
    for (message, content) in inbox.iter().filter_map(|m| {
        if let Message::Command(content) = &m.content {
            Some((m, content))
        } else {
            None
        }
    }) {
        let mut client = value_or_continue!(clients.iter_mut().find(|c| c.id == message.from));

        if content[0..=2] == [IAC, SB, NAWS] {
            let width = content.get(4).map(|a| *a as u16).unwrap_or(80);

            if width > 0 {
                client.width = width;
            }
        }
    }
}

pub fn send_prompt(
    mut events: EventReader<Prompt>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Character, &Attributes, &State)>,
    npcs: Query<&Depiction, With<Npc>>,
) {
    for prompt in events.iter() {
        let (client, character, attributes, state) =
            value_or_continue!(players.iter().find(|(c, _, _, _)| c.id == prompt.client_id));

        let mut parts: Vec<String> = vec![];

        parts.push(paint!(
            "[{}/<fg.red>{}</>]",
            state.health,
            attributes.max_health()
        ));

        let target = match character.state {
            CharacterState::Idle => None,
            CharacterState::Combat(target) => {
                if let Ok(depiction) = npcs.get(target) {
                    Some(depiction.name.clone())
                } else {
                    None
                }
            }
        };

        if let Some(target) = target {
            parts.push(format!("({target})"));
        }

        parts.push("->".into());

        outbox.send_text(client.id, parts.join(" "));
    }
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
