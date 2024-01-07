use std::fmt::{self, Display, Formatter};

use bevy::prelude::*;
use bevy_nest::prelude::*;

#[derive(Clone, Debug, PartialEq)]
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

impl ChatChannel {
    pub fn color(&self) -> String {
        match self {
            Self::Chat => "cyan".into(),
            Self::Novice => "green".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Advance,
    Announce(String),
    Attack(Option<String>),
    Block,
    Chat((ChatChannel, String)),
    Close(Option<String>),
    Config((Option<String>, Option<String>)),
    Describe(Option<String>),
    Dodge,
    Drop((String, bool)),
    Emote(String),
    Enter(Option<String>),
    Examine(String),
    Inventory,
    Look(Option<String>),
    Map,
    Menu(String),
    Movement((String, bool)),
    Open(Option<String>),
    Place((String, String)),
    Quit,
    Retreat,
    Roll(String),
    Say(String),
    Scan((bool, Option<String>)),
    Sit(Option<String>),
    Stand,
    Take((String, bool, Option<String>)),
    Time,
    UseSkill((String, Option<String>)),
    Who,
    Yell(String),
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    WrongCommand,
    InvalidArguments(String),
    UnknownCommand,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongCommand => unreachable!(),
            Self::InvalidArguments(suggestion) => write!(f, "{suggestion}"),
            Self::UnknownCommand => write!(f, "You don't know how to do that."),
        }
    }
}

/// A command sent from the client to the server.
#[derive(Event, Clone, Debug)]
pub struct ParsedCommand {
    pub from: ClientId,
    pub command: Command,
}

/// A command sent from the server to the server as if
/// it was sent from a client.
#[derive(Event, Debug)]
pub struct ProxyCommand(pub ParsedCommand);
