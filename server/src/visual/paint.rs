use std::sync::{
    atomic::{AtomicBool, Ordering},
    OnceLock,
};

use colored::{ColoredString, Colorize};
use regex::Regex;

static REGEX: OnceLock<Regex> = OnceLock::new();
static REGEX_ATTR: OnceLock<Regex> = OnceLock::new();

static ENABLED: AtomicBool = AtomicBool::new(true);

#[derive(Clone, Copy)]
pub enum Color {
    Enemy,
    Item,
    Npc,
    Player,
    Transition,
}

impl Color {
    pub fn value(&self) -> &str {
        match self {
            Color::Enemy => "red",
            Color::Item => "yellow",
            Color::Npc => "blue",
            Color::Player => "cyan",
            Color::Transition => "green",
        }
    }
}

#[allow(dead_code)]
pub fn toggle(enabled: bool) {
    ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn style(text: &str) -> String {
    if !ENABLED.load(Ordering::Relaxed) {
        return strip(text);
    }

    let regex = REGEX.get_or_init(|| {
        Regex::new(r"<(?P<attrs>((bg|fg|s)\.\S+\s*)+)>(?P<content>[^>]+)</>").unwrap()
    });

    let regex_attr =
        REGEX_ATTR.get_or_init(|| Regex::new(r"(?P<attr>(bg|fg|s))\.(?P<value>\S+)").unwrap());

    regex
        .replace_all(text, |cap: &regex::Captures| {
            let mut styled = ColoredString::from(&cap["content"]);

            for attr in regex_attr.captures_iter(&cap["attrs"]) {
                let value = match &attr["value"] {
                    "enemy" => Color::Enemy.value(),
                    "item" => Color::Item.value(),
                    "npc" => Color::Npc.value(),
                    "player" => Color::Player.value(),
                    "transition" => Color::Transition.value(),
                    _ => &attr["value"],
                };

                styled = match &attr["attr"] {
                    "fg" => styled.color(value),
                    "bg" => styled.on_color(value),
                    "s" => match &attr["value"] {
                        "bold" => styled.bold(),
                        "dimmed" => styled.dimmed(),
                        "italic" => styled.italic(),
                        "underline" => styled.underline(),
                        "blink" => styled.blink(),
                        "reverse" => styled.reverse(),
                        "hidden" => styled.hidden(),
                        "strikethrough" => styled.strikethrough(),
                        _ => styled.normal(),
                    },
                    _ => styled.normal(),
                }
            }

            styled.to_string()
        })
        .to_string()
}

pub fn strip(text: &str) -> String {
    let regex = REGEX.get_or_init(|| {
        Regex::new(r"<(?P<attrs>((bg|fg|s)\.\S+\s*)+)>(?P<content>[^>]+)</>").unwrap()
    });

    regex.replace_all(text, "$content").to_string()
}

#[macro_export]
macro_rules! paint {
    ($fmt:literal $(, $args:expr)* $(,)?) => {{
        let formatted = format!($fmt $(, $args)*);

        $crate::visual::paint::style(&formatted)
    }}
}
