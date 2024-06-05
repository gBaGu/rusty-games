use std::time::Duration;
use bevy::prelude::*;
use rand::prelude::*;

use super::Bot;
use crate::game::{Authority, CurrentGame, GameType, LocalGameTurn, NetworkGameTurn, Position};

pub fn make_turn(
    mut bot: Query<&mut Bot>,
    mut network_turn_data: EventWriter<NetworkGameTurn>,
    mut local_turn_data: EventWriter<LocalGameTurn>,
    game: Res<CurrentGame>,
    time: Res<Time>,
) {
    for mut bot in bot.iter_mut() {
        let Some(next_player) = game.get_next_player() else {
            continue;
        };
        let auth = Authority::Bot(bot.id());
        if next_player.auth() == auth {
            let mut rng = thread_rng();
            if bot.timer().paused() {
                let milliseconds = rng.gen_range(500..1500);
                bot.start_timer(Duration::from_millis(milliseconds));
                continue;
            }

            if bot.timer_mut().tick(time.delta()).just_finished() {
                bot.reset_timer();
                let empty_cells: Vec<_> = game
                    .board()
                    .iter()
                    .enumerate()
                    .map(|(i, row)| row.iter().enumerate().map(move |(j, cell)| (i, j, cell)))
                    .flatten()
                    .filter(|(_, _, cell)| cell.is_none())
                    .collect();
                let index = rng.gen_range(0..empty_cells.len());
                let rand_cell = empty_cells[index];
                let pos = Position::new(rand_cell.0 as u32, rand_cell.1 as u32);
                match game.game_type() {
                    GameType::Network(id) => {
                        network_turn_data.send(NetworkGameTurn {
                            game_id: id,
                            auth,
                            pos,
                        });
                    }
                    GameType::Local => {
                        local_turn_data.send(LocalGameTurn { auth, pos });
                    }
                };
            }
        }
    }
}
