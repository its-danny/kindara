use bevy::{prelude::*, utils::HashMap};
use indefinite::indefinite;
use inflector::string::pluralize::to_plural;

use super::components::Item;

pub fn item_matches_query(entity: &Entity, item: &Item, query: &str) -> bool {
    if let Some(idx) = query
        .starts_with('#')
        .then(|| query.trim_start_matches('#').parse::<u32>().ok())
        .flatten()
    {
        idx == entity.index()
    } else {
        item.name.eq_ignore_ascii_case(query)
            || item.short_name.eq_ignore_ascii_case(query)
            || item.tags.contains(&query.to_lowercase())
    }
}

pub fn item_name_list(item_names: &[String]) -> String {
    let count_map: HashMap<String, u16> =
        item_names
            .iter()
            .cloned()
            .fold(HashMap::new(), |mut map, name| {
                *map.entry(name).or_default() += 1;

                map
            });

    let mut counted_names = count_map
        .iter()
        .map(|(name, count)| {
            if *count > 1 {
                format!("{} {}", count, to_plural(name))
            } else {
                indefinite(name)
            }
        })
        .collect::<Vec<_>>();

    counted_names.sort();

    match counted_names.len() {
        0 => String::new(),
        1 => counted_names[0].clone(),
        2 => format!("{} and {}", counted_names[0], counted_names[1]),
        _ => {
            let last = counted_names.pop().unwrap_or_default();

            format!("{}, and {}", counted_names.join(", "), last)
        }
    }
}
