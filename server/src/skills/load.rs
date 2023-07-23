use std::{env, path::PathBuf};

use bevy::{prelude::*, utils::HashMap};

use crate::skills::resources::Skill;

use super::resources::Skills;

pub fn load_skills(mut skills: ResMut<Skills>) {
    let path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("assets/skills");

    debug!("Loading skills from: {:?}", path);

    let defs = std::fs::read_dir(path)
        .expect("Failed to read skills directory")
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().unwrap() == "ron" {
                let def: Skill = ron::from_str(
                    std::fs::read_to_string(path)
                        .expect("Failed to load skill def")
                        .as_str(),
                )
                .expect("Failed to parse skill def");

                Some((def.name.clone(), def))
            } else {
                None
            }
        })
        .collect::<HashMap<String, Skill>>();

    skills.0 = defs;
}
