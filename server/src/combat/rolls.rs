use caith::Roller;

pub fn roll_total(roll: &str) -> i64 {
    let roller = Roller::new(roll).unwrap();
    let roll = roller.roll().unwrap();
    let result = roll.as_single().unwrap();

    result.get_total()
}

pub fn roll_for_attack_quality() -> i64 {
    roll_total("2d10")
}

pub fn roll_for_dodge() -> i64 {
    roll_total("1d10")
}
