use bevy::{asset::FileAssetIo, prelude::*, utils::HashMap};
use walkdir::WalkDir;

use super::resources::{
    Condition, Conditions, DamageKind, DamageKinds, Masteries, Mastery, Resistance, Resistances,
    Skill, Skills,
};

pub fn load_damage_kinds(mut damage_kinds: ResMut<DamageKinds>) {
    let path = FileAssetIo::get_base_path().join("assets/damage-types.ron");

    debug!("Loading damage types from: {:?}", path);

    let defs = ron::from_str::<Vec<DamageKind>>(
        std::fs::read_to_string(path)
            .expect("Failed to load damage types")
            .as_str(),
    )
    .map(|defs| {
        defs.into_iter()
            .map(|def| (def.id.clone(), def))
            .collect::<HashMap<String, DamageKind>>()
    })
    .expect("Failed to parse damage types");

    damage_kinds.0 = defs;
}

pub fn load_resistances(mut resistances: ResMut<Resistances>) {
    let path = FileAssetIo::get_base_path().join("assets/resistances.ron");

    debug!("Loading resistances from: {:?}", path);

    let defs = ron::from_str::<Vec<Resistance>>(
        std::fs::read_to_string(path)
            .expect("Failed to load resistances")
            .as_str(),
    )
    .map(|defs| {
        defs.into_iter()
            .map(|def| (def.id.clone(), def))
            .collect::<HashMap<String, Resistance>>()
    })
    .expect("Failed to parse resistances");

    resistances.0 = defs;
}

pub fn load_masteries(mut masteries: ResMut<Masteries>) {
    let path = FileAssetIo::get_base_path().join("assets");

    debug!("Loading masteries from: {:?}", path);

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with("mastery.ron")
        })
    {
        let path = entry.path();

        let contents = std::fs::read_to_string(path)
            .expect("Failed to load mastery definition")
            .as_str()
            .to_string();

        let parsed = ron::from_str::<Mastery>(contents.as_str())
            .expect("Failed to parse mastery definition");

        debug!("Loaded mastery: {:?}", parsed.id);

        masteries.0.insert(parsed.id.clone(), parsed);
    }
}

pub fn load_skills(mut skills: ResMut<Skills>) {
    let path = FileAssetIo::get_base_path().join("assets");

    debug!("Loading skills from: {:?}", path);

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".skill.ron")
        })
    {
        let path = entry.path();

        let contents = std::fs::read_to_string(path)
            .expect("Failed to load skill definition")
            .as_str()
            .to_string();

        let parsed =
            ron::from_str::<Skill>(contents.as_str()).expect("Failed to parse skill definition");

        debug!("Loaded skill: {:?}", parsed.id);

        skills.0.insert(parsed.id.clone(), parsed);
    }
}

pub fn load_conditions(mut conditions: ResMut<Conditions>) {
    let path = FileAssetIo::get_base_path().join("assets");

    debug!("Loading conditions from: {:?}", path);

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            e.path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .ends_with(".condition.ron")
        })
    {
        let path = entry.path();

        let contents = std::fs::read_to_string(path)
            .expect("Failed to load condition definition")
            .as_str()
            .to_string();

        let parsed = ron::from_str::<Condition>(contents.as_str())
            .expect("Failed to parse condition definition");

        debug!("Loaded condition: {:?}", parsed.id);

        conditions.0.insert(parsed.id.clone(), parsed);
    }
}
