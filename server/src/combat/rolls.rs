use caith::Roller;

use crate::skills::resources::{Action, RelevantStat, Skill};

use super::components::{Attributes, State};

pub enum HitResponse {
    Missed,
    Hit,
}

pub fn roll_hit() -> HitResponse {
    let roller = Roller::new("2d10").unwrap();
    let roll = roller.roll().unwrap();
    let quality = roll.as_single().unwrap().get_total();

    let roller = Roller::new("1d10").unwrap();
    let roll = roller.roll().unwrap();
    let dodge = roll.as_single().unwrap().get_total();

    if quality < dodge {
        HitResponse::Missed
    } else {
        HitResponse::Hit
    }
}

pub fn apply_actions(skill: &Skill, attributes: &Attributes, state: &mut State) {
    for action in &skill.actions {
        match action {
            Action::ApplyDamage(roll) => {
                let roller = Roller::new(roll).unwrap();
                let roll = roller.roll().unwrap();
                let mut damage = roll.as_single().unwrap().get_total() as u32;

                damage += match &skill.stat {
                    RelevantStat::Strength => attributes.strength,
                    RelevantStat::Dexterity => attributes.dexterity,
                    RelevantStat::Intelligence => attributes.intelligence,
                };

                state.apply_damage(damage);
            }
        }
    }
}
