use std::time::Duration;

use bevy::prelude::*;
use game_server::game::Game;
use rand::{thread_rng, Rng};

use super::{
    Delay, LocalGame, PendingAction, PlayerActionInitialized, QLearningModel, QLearningStrategy,
    RandomStrategy,
};
use crate::game::{BotAuthority, BotReady, CurrentPlayer, PlayerPosition};

const MIN_ACTION_DELAY: u64 = 500;
const MAX_ACTION_DELAY: u64 = 1500;

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

/// Listen for [`BotReady`] event and choose random action from currently available
pub fn random_bot(
    game: Query<(Entity, &LocalGame), Without<PendingAction>>,
    mut bot_ready: EventReader<BotReady>,
    mut action_initialized: EventWriter<PlayerActionInitialized>,
) {
    for event in bot_ready.read() {
        let Ok((game_entity, game)) = game.get(event.game()) else {
            continue;
        };
        let mut rng = thread_rng();
        let empty_cells: Vec<_> = game
            .board()
            .all_indexed()
            .filter_map(|(index, cell)| if cell.is_none() { Some(index) } else { None })
            .collect();
        let index = rng.gen_range(0..empty_cells.len());
        let action = empty_cells[index];
        println!("action initialized by a random bot: {}", action);
        action_initialized.send(PlayerActionInitialized::new(
            game_entity,
            event.player_position(),
            action,
        ));
    }
}

/// Listen for [`BotReady`] event and get action from [`QLearningModel`].
pub fn q_learning_bot(
    game: Query<(Entity, &LocalGame), Without<PendingAction>>,
    bot: Query<&QLearningStrategy, With<BotAuthority>>,
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
        let Some(action) = model.get_move(strategy.difficulty(), game.board()) else {
            println!("unable to get action from q-learning model");
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
