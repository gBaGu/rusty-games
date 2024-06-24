use bevy::prelude::*;

use super::Bot;
use crate::game::{Authority, CurrentGame, LocalGameTurn, NetworkGameTurn};

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
            if !bot.waiting_delay() {
                bot.start_delay();
                continue;
            }

            if bot.tick_delay(time.delta()).just_finished() {
                bot.reset_delay();
                let pos = bot.get_move(game.board());
                if let Some(pos) = pos {
                    game.trigger_turn(&mut network_turn_data, &mut local_turn_data, auth, pos);
                } else {
                    println!("failed to get move from bot");
                }
            }
        }
    }
}
