use crate::game::components::PendingActionStatus;
use crate::grpc::RpcResultReady;
use crate::interface::GameReadyToExit;
use bevy::prelude::*;
use game_server::game::{FinishedState, GameState};
use game_server::proto;
use std::ops::Deref;

use super::{
    ActiveGame, CurrentPlayer, Draw, NetworkGame, PlayerPosition, PlayerWon, StateUpdated,
    TurnStart, Winner,
};

/// Receive reply for MakeTurn rpc and if it's successful update [`PendingActionStatus`] component.
/// Does not depend on specific game type
pub fn handle_make_turn(
    mut game: Query<&mut PendingActionStatus, (With<NetworkGame>, With<ActiveGame>)>,
    mut make_turn_reply: EventReader<RpcResultReady<proto::MakeTurnReply>>,
) {
    let Ok(mut status) = game.get_single_mut() else {
        return;
    };
    for event in make_turn_reply.read() {
        if status.is_confirmed() {
            return;
        }
        *status = PendingActionStatus::NotConfirmed;
        let _ = match event.deref() {
            Ok(response) => {
                if let Some(new_state) = &response.get_ref().game_state {
                    match GameState::try_from(new_state.clone()) {
                        Ok(state) => state,
                        Err(err) => {
                            println!("failed to convert game state: {}", err);
                            continue;
                        }
                    }
                } else {
                    println!("MakeTurn returned empty response");
                    continue;
                }
            }
            Err(err) => {
                println!("MakeTurn request failed: {}", err);
                continue;
            }
        };
        *status = PendingActionStatus::Confirmed;
    }
}

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

pub fn handle_draw(mut draw: EventReader<Draw>, mut ready_to_exit: EventWriter<GameReadyToExit>) {
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
        player
            .iter()
            .filter(|(.., p)| p.get() == event.game())
            .for_each(|(player, &pos, _)| {
                let mut player_cmds = commands.entity(player);
                player_cmds.remove::<CurrentPlayer>();
                if event.player() == *pos {
                    player_cmds.insert(Winner);
                }
            })
    }
}
