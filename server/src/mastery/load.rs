use bevy::{asset::FileAssetIo, prelude::*, utils::HashMap};

use crate::mastery::resources::Mastery;

use super::resources::Masteries;

pub fn load_masteries(mut masteries: ResMut<Masteries>) {
    let path = FileAssetIo::get_base_path().join("assets/masteries");

    debug!("Loading masteries from: {:?}", path);

    let defs = std::fs::read_dir(path)
        .expect("Failed to read masteries directory")
        .filter_map(|entry| {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.extension().unwrap() == "ron" {
                let def: Mastery = ron::from_str(
                    std::fs::read_to_string(path)
                        .expect("Failed to load mastery def")
                        .as_str(),
                )
                .expect("Failed to parse mastery def");

                Some((def.name.to_lowercase(), def))
            } else {
                None
            }
        })
        .collect::<HashMap<String, Mastery>>();

    masteries.0 = defs;
}
