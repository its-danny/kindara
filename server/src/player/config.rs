use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Copy, Clone, Default, Serialize, Deserialize)]
pub struct CharacterConfig {
    pub brief: bool,
}

impl CharacterConfig {
    pub fn get(&self, option: &str) -> Result<(Value, &'static str), &'static str> {
        match option {
            "brief" => Ok((
                self.brief.into(),
                "If enabled, you will only see room names when moving.",
            )),
            _ => Err("Invalid option."),
        }
    }

    pub fn set(&mut self, option: &str, value: &str) -> Result<(), &'static str> {
        match option {
            "brief" => {
                self.brief = value
                    .parse()
                    .map_err(|_| "Value must be `true` or `false`.")?;

                Ok(())
            }
            _ => Err("Invalid option."),
        }
    }
}
