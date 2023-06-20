use bevy::utils::HashMap;
use indefinite::indefinite_article_only;
use inflector::string::pluralize::to_plural;

use crate::visual::paint::Color;

pub fn name_list(names: &[String], color: Option<Color>, include_indefinite: bool) -> String {
    let count_map: HashMap<String, u16> =
        names.iter().cloned().fold(HashMap::new(), |mut map, name| {
            *map.entry(name).or_default() += 1;

            map
        });

    let color_tag = color.map_or_else(
        || String::with_capacity(0),
        |color| format!("<fg.{}>", color.value()),
    );
    let reset_tag = color.map_or_else(|| String::with_capacity(0), |_| String::from("</>"));

    let mut counted_names = count_map
        .iter()
        .map(|(name, count)| {
            let mut output = String::with_capacity(name.len() + 10);

            if *count > 1 {
                output.push_str(&format!("{} ", count));
                output.push_str(&color_tag);
                output.push_str(&to_plural(name));
            } else {
                if include_indefinite {
                    output.push_str(&indefinite_article_only(name));
                    output.push(' ');
                }

                output.push_str(&color_tag);
                output.push_str(name);
            }

            output.push_str(&reset_tag);

            output
        })
        .collect::<Vec<_>>();

    counted_names.sort();

    let mut result = String::new();

    match counted_names.len() {
        0 => result,
        1 => counted_names[0].clone(),
        2 => {
            result.push_str(&counted_names[0]);
            result.push_str(" and ");
            result.push_str(&counted_names[1]);

            result
        }
        _ => {
            let last = counted_names.pop().unwrap_or_default();

            result.push_str(&counted_names.join(", "));
            result.push_str(", and ");
            result.push_str(&last);

            result
        }
    }
}
