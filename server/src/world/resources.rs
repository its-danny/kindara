use std::fmt::Display;

use bevy::prelude::*;
use chrono::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Resource)]
pub struct SaveTimer(pub Timer);

#[derive(Debug, Default, Serialize, Deserialize, Resource)]
pub struct WorldState {
    pub characters: Vec<WorldStateCharacter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldStateCharacter {
    pub id: i64,
    pub tile: String,
    pub inventory: Vec<String>,
}

#[derive(Default, Resource)]
pub struct WorldTime {
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl Display for WorldTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "It's {}, {}", self.date_string(), self.time_string())
    }
}

impl WorldTime {
    pub fn update(&mut self) {
        let now = Utc::now();

        let game_ratio = 4;
        let minutes_per_day = 24 * 60;

        let total_minutes_passed = now.minute() + now.hour() * 60 + now.day() * minutes_per_day;
        let total_game_minutes_passed = total_minutes_passed * game_ratio;

        let game_day = total_game_minutes_passed / minutes_per_day;
        let game_hour = (total_game_minutes_passed % minutes_per_day) / 60;
        let game_minute = (total_game_minutes_passed % minutes_per_day) % 60;

        self.year = now.year() as u32 - 2020;
        self.month = now.month();
        self.day = game_day;
        self.hour = game_hour;
        self.minute = game_minute;
    }

    pub fn date_string(&self) -> String {
        format!(
            "day {} of month {}, year {}",
            self.day, self.month, self.year
        )
    }

    pub fn time_string(&self) -> String {
        format!(
            "{:02}:{:02}{}",
            self.hour,
            self.minute,
            if self.hour < 12 { "am" } else { "pm" }
        )
    }

    pub fn is_dawn(&self) -> bool {
        self.hour > 5 && self.hour < 7
    }

    pub fn is_day(&self) -> bool {
        self.hour > 7 && self.hour < 19
    }

    pub fn is_dusk(&self) -> bool {
        self.hour > 19 && self.hour < 21
    }

    pub fn is_night(&self) -> bool {
        self.hour > 21 || self.hour < 5
    }
}
