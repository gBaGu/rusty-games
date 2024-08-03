use std::time::Duration;

use bevy::prelude::*;
use game_server::game::grid::GridIndex;
use game_server::game::Game;
use rand::{thread_rng, Rng};

use super::{Delay, LocalGame, PendingAction, PlayerActionInitialized, QLearningModel, Strategy};
use crate::game::{BotAuthority, BotReady, CurrentPlayer, PlayerPosition, TTTBoard};

const MIN_ACTION_DELAY: u64 = 500;
const MAX_ACTION_DELAY: u64 = 1500;

fn get_random_action(board: &TTTBoard) -> Option<GridIndex> {
    let empty_cells: Vec<_> = board
        .all_indexed()
        .filter_map(|(index, cell)| if cell.is_none() { Some(index) } else { None })
        .collect();
    if empty_cells.is_empty() {
        return None;
    }
    let mut rng = thread_rng();
    let index = rng.gen_range(0..empty_cells.len());
    Some(empty_cells[index])
}

pub fn start_delay(mut bot: Query<&mut Delay, (With<BotAuthority>, Added<CurrentPlayer>)>) {
    let mut rng = thread_rng();
    for mut delay in bot.iter_mut() {
        let milliseconds = rng.gen_range(MIN_ACTION_DELAY..MAX_ACTION_DELAY);
        println!("starting bot delay ({}ms)", milliseconds);
        delay.reset();
        delay.start(Duration::from_millis(milliseconds));
    }
}

/// Spend some time (between `MIN_ACTION_DELAY` and `MAX_ACTION_DELAY`) and emit [`BotReady`].
/// For bot entities that doesn't have [`Delay`] component emit [`BotReady`] immediately.
pub fn delay(
    mut bot: Query<
        (Entity, Option<&mut Delay>, &Parent, &PlayerPosition),
        (With<BotAuthority>, With<CurrentPlayer>),
    >,
    mut bot_ready: EventWriter<BotReady>,
    time: Res<Time>,
) {
    for (bot_entity, delay, parent, &position) in bot.iter_mut() {
        let Some(mut delay) = delay else {
            bot_ready.send(BotReady::new(bot_entity, parent.get(), *position));
            continue;
        };
        if delay.tick(time.delta()).just_finished() {
            println!("bot is ready to make a move");
            bot_ready.send(BotReady::new(bot_entity, parent.get(), *position));
        }
    }
}

pub fn initialize_action(
    game: Query<(Entity, &LocalGame), Without<PendingAction>>,
    bot: Query<&Strategy, With<BotAuthority>>,
    mut bot_ready: EventReader<BotReady>,
    mut action_initialized: EventWriter<PlayerActionInitialized>,
    model: Res<QLearningModel>,
) {
    for event in bot_ready.read() {
        let Ok(strategy) = bot.get(event.bot()) else {
            continue;
        };
        let Ok((game_entity, game)) = game.get(event.game()) else {
            continue;
        };
        let action = match strategy {
            Strategy::Random => get_random_action(game.board()),
            Strategy::QLearning(difficulty) => model.get_move(*difficulty, game.board()),
        };
        let Some(action) = action else {
            println!("unable to get action from bot strategy");
            continue;
        };
        println!("action initialized by a q-learning bot: {}", action);
        action_initialized.send(PlayerActionInitialized::new(
            game_entity,
            event.player_position(),
            action,
        ));
    }
}