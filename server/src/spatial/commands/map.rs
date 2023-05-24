use bevy::prelude::*;
use bevy_nest::prelude::*;
use regex::Regex;

use crate::{
    player::components::{Character, Client},
    spatial::components::{Position, Tile},
    visual::components::Sprite,
    world::resources::TileMap,
};

// USAGE: (map|m)
pub fn map(
    tile_map: Res<TileMap>,
    mut inbox: EventReader<Inbox>,
    mut outbox: EventWriter<Outbox>,
    players: Query<(&Client, &Position), With<Character>>,
    tiles: Query<&Sprite, With<Tile>>,
) {
    let regex = Regex::new(r"^(map|m)$").unwrap();

    for (message, _) in inbox.iter().filter_map(|message| match &message.content {
        Message::Text(text) if regex.is_match(text) => Some((message, text)),
        _ => None,
    }) {
        let Some((client, player_position)) = players.iter().find(|(c, _)| c.0 == message.from) else {
            return;
        };

        let width = 64;
        let height = 16;

        let mut map = vec![vec![' '; width]; height];

        let start_x = player_position.coords.x - (width as i32 / 2);
        let end_x = player_position.coords.x + (width as i32 / 2);
        let start_y = player_position.coords.y - (height as i32 / 2);
        let end_y = player_position.coords.y + (height as i32 / 2);

        for x in start_x..=end_x {
            for y in start_y..=end_y {
                if x == player_position.coords.x && y == player_position.coords.y {
                    map[(y - start_y) as usize][(x - start_x) as usize] = '@';
                } else if let Some(sprite) = tile_map
                    .get(
                        player_position.zone,
                        IVec3::new(x, y, player_position.coords.z),
                    )
                    .and_then(|e| tiles.get(*e).ok())
                {
                    map[(y - start_y) as usize][(x - start_x) as usize] =
                        sprite.character.chars().next().unwrap_or(' ');
                }
            }
        }

        let display = map
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");

        outbox.send_text(client.0, format!("{}\n{}", player_position.zone, display));
    }
}
