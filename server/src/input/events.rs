use bevy_nest::server::ClientId;

#[derive(Clone, Debug)]
pub enum Command {
    Config((Option<String>, Option<String>)),
    Drop((String, bool)),
    Enter(Option<String>),
    Inventory,
    Look,
    Map,
    Movement(String),
    Say(String),
    Take((String, bool)),
    Teleport((String, (i32, i32, i32))),
    Who,
}

#[derive(Clone, Debug)]
pub struct ParsedCommand {
    pub from: ClientId,
    pub command: Command,
}

pub struct ProxyCommand(pub ParsedCommand);
