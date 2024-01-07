use bevy::{prelude::*, utils::HashMap};

#[derive(Resource, Default)]
pub struct Scripts(pub HashMap<String, String>);
