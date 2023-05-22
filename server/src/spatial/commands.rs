use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::player::components::{Character, Client};

use super::components::{Position, Tile};

pub(super) fn map(
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position), With<Character>>,
    tiles: Query<(&Position, &Tile)>,
) {
    let regex = Regex::new("^(map|m)$").unwrap();

    for message in inbox
        .iter()
        .filter(|message| matches!(&message.content, Message::Text(text) if regex.is_match(text)))
    {
        let Some((_, position)) = players.iter().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        let width = 80;
        let height = 16;

        let mut map = vec![vec![' '; width]; height];

        let start_x = position.0.x - (width as i32 / 2);
        let end_x = position.0.x + (width as i32 / 2);
        let start_y = position.0.y - (height as i32 / 2);
        let end_y = position.0.y + (height as i32 / 2);

        for x in start_x..=end_x {
            for y in start_y..=end_y {
                if x == position.0.x && y == position.0.y {
                    map[(y - start_y) as usize][(x - start_x) as usize] = '@';
                } else if tiles
                    .iter()
                    .any(|(position, _)| position.0 == IVec3::new(x, y, 0))
                {
                    map[(y - start_y) as usize][(x - start_x) as usize] = 'X';
                }
            }
        }

        let display = map
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<String>>()
            .join("\n");

        outbox.send_text(message.from, display);
    }
}
