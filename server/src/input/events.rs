use bevy_nest::server::ClientId;

pub enum Command {
    Config((Option<String>, Option<String>)),
    Enter(Option<String>),
    Look,
    Map,
    Movement(String),
    Say(String),
    Teleport((String, (i32, i32, i32))),
    Who,
}

pub struct ParsedCommand {
    pub from: ClientId,
    pub command: Command,
}
