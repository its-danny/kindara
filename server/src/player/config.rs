use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Copy, Clone, Default, Serialize, Deserialize)]
pub struct CharacterConfig {
    pub brief: bool,
}

impl CharacterConfig {
    pub fn get(&self, option: &str) -> Option<(&'static str, Value)> {
        match option {
            "brief" => Some((
                "If enabled, you will only see room names when moving.",
                self.brief.into(),
            )),
            _ => None,
        }
    }

    pub fn set(&mut self, option: &str, value: &str) -> Result<(), &'static str> {
        match option {
            "brief" => {
                self.brief = value.parse().map_err(|_| "Invalid value")?;
            }
            _ => return Err("Invalid option"),
        }

        Ok(())
    }
}
