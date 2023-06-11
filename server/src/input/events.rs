use std::fmt::{self, Display, Formatter};

use bevy_nest::server::ClientId;

#[derive(Clone, Debug)]
pub enum ChatChannel {
    Chat,
    Novice,
}

impl Display for ChatChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Chat => write!(f, "chat"),
            Self::Novice => write!(f, "novice"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Chat((ChatChannel, String)),
    Config((Option<String>, Option<String>)),
    Drop((String, bool)),
    Emote(String),
    Enter(Option<String>),
    Inventory,
    Look(Option<String>),
    Map,
    Movement(String),
    Place((String, String)),
    Say(String),
    Take((String, bool, Option<String>)),
    Teleport((String, (i32, i32, i32))),
    Who,
    Yell(String),
}

#[derive(Clone, Debug)]
pub struct ParsedCommand {
    pub from: ClientId,
    pub command: Command,
}

pub struct ProxyCommand(pub ParsedCommand);
