use bevy::prelude::*;
use game_server::game::{FinishedState, GameState};
use crate::interface::GameReadyToExit;

use super::{CurrentPlayer, Draw, PlayerPosition, PlayerWon, StateUpdated, TurnStart, Winner};

pub fn handle_state_updated(
    mut state_updated: EventReader<StateUpdated>,
    mut turn_start: EventWriter<TurnStart>,
    mut player_won: EventWriter<PlayerWon>,
    mut draw: EventWriter<Draw>,
) {
    for event in state_updated.read() {
        match event.state() {
            GameState::Turn(next_player) => {
                println!("turn start: {:?}", event.game());
                turn_start.send(TurnStart::new(event.game(), next_player));
            }
            GameState::Finished(FinishedState::Win(winner)) => {
                println!("win: {:?}", event.game());
                player_won.send(PlayerWon::new(event.game(), winner));
            }
            GameState::Finished(FinishedState::Draw) => {
                println!("draw: {:?}", event.game());
                draw.send(Draw::new(event.game()));
            }
        }
    }
}

pub fn update_current_player(
    mut commands: Commands,
    player: Query<(Entity, &PlayerPosition, &Parent)>,
    mut turn_start: EventReader<TurnStart>,
) {
    for event in turn_start.read() {
        player
            .iter()
            .filter(|(.., p)| event.game() == p.get())
            .for_each(|(player_entity, &position, _)| {
                let mut player_cmds = commands.entity(player_entity);
                if *position == event.player() {
                    println!("set current player: {}", event.player());
                    player_cmds.insert(CurrentPlayer);
                } else {
                    player_cmds.remove::<CurrentPlayer>();
                }
            });
    }
}

pub fn handle_draw(
    mut draw: EventReader<Draw>,
    mut ready_to_exit: EventWriter<GameReadyToExit>,
) {
    for event in draw.read() {
        ready_to_exit.send(GameReadyToExit::new(event.game()));
    }
}

pub fn handle_win(
    mut commands: Commands,
    player: Query<(Entity, &PlayerPosition, &Parent)>,
    mut player_won: EventReader<PlayerWon>,
) {
    for event in player_won.read() {
        player.iter().filter(|(.., p)| p.get() == event.game()).for_each(|(player, &pos, _)| {
            let mut player_cmds = commands.entity(player);
            player_cmds.remove::<CurrentPlayer>();
            if event.player() == *pos {
                player_cmds.insert(Winner);
            }
        })
    }
}
