use bevy::prelude::*;
use bevy_proto::prelude::*;

#[derive(Component, Reflect, FromReflect)]
pub struct Sprite {
    pub character: String,
}

#[derive(Component, Schematic, Reflect, FromReflect)]
#[reflect(Schematic)]
pub struct Depiction {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub visible: bool,
}

impl Depiction {
    pub fn matches_query(&self, entity: &Entity, query: &str) -> bool {
        if let Some(idx) = query
            .starts_with('#')
            .then(|| query.trim_start_matches('#').parse::<u32>().ok())
            .flatten()
        {
            idx == entity.index()
        } else {
            self.name.eq_ignore_ascii_case(query)
                || self.short_name.eq_ignore_ascii_case(query)
                || self.tags.contains(&query.to_lowercase())
        }
    }
}
