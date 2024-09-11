use std::ops::Deref;

use bevy::prelude::*;
use game_server::{core, proto};

use super::{
    ActiveGame, CurrentPlayer, CurrentUser, Draw, NetworkGame, PlayerPosition, PlayerWon,
    StateUpdated, TurnStart, UserAuthority, Winner,
};
use crate::game::components::PendingActionStatus;
use crate::grpc::RpcResultReady;
use crate::interface::GameReadyToExit;
use crate::UserIdChanged;

/// Receive reply for MakeTurn rpc and if it's successful update [`PendingActionStatus`] component.
/// Does not depend on specific game type.
pub fn confirm_pending_action(
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
                    match core::GameState::try_from(new_state.clone()) {
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

/// Receive [`StateUpdated`] event and send [`TurnStart`], [`PlayerWon`] or [`Draw`]
/// depending on a new state.
pub fn handle_state_updated(
    mut state_updated: EventReader<StateUpdated>,
    mut turn_start: EventWriter<TurnStart>,
    mut player_won: EventWriter<PlayerWon>,
    mut draw: EventWriter<Draw>,
) {
    for event in state_updated.read() {
        match event.state() {
            core::GameState::Turn(next_player) => {
                println!("turn start: {:?}", event.game());
                turn_start.send(TurnStart::new(event.game(), next_player));
            }
            core::GameState::Finished(core::FinishedState::Win(winner)) => {
                println!("win: {:?}", event.game());
                player_won.send(PlayerWon::new(event.game(), winner));
            }
            core::GameState::Finished(core::FinishedState::Draw) => {
                println!("draw: {:?}", event.game());
                draw.send(Draw::new(event.game()));
            }
        }
    }
}

/// Receives [`TurnStart`] event and updates [`CurrentPlayer`] accordingly.
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

/// Receives [`Draw`] event, clears [`CurrentPlayer`] tag for players
/// and sends [`GameReadyToExit`] event because currently
/// there is nothing to do after the game is ended with a draw.
pub fn handle_draw(
    mut commands: Commands,
    player: Query<(Entity, &Parent), With<CurrentPlayer>>,
    mut draw: EventReader<Draw>,
    mut ready_to_exit: EventWriter<GameReadyToExit>,
) {
    for event in draw.read() {
        for (player_entity, _) in player.iter().filter(|(.., p)| p.get() == event.game()) {
            let mut player_cmds = commands.entity(player_entity);
            player_cmds.remove::<CurrentPlayer>();
        }
        ready_to_exit.send(GameReadyToExit::new(event.game()));
    }
}

/// Receives [`PlayerWon`] event, clears [`CurrentPlayer`] tag for players
/// and inserts [`Winner`] tag into the entity of a player that won the game.
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

/// Whenever user id is changed in settings insert/remove [`CurrentUser`] for every player
/// according to his id.
pub fn update_current_user(
    mut commands: Commands,
    player: Query<(Entity, &UserAuthority)>,
    mut user_id_changed: EventReader<UserIdChanged>,
) {
    if let Some(event) = user_id_changed.read().last() {
        for (player_entity, &user_authority) in player.iter() {
            let mut player_cmds = commands.entity(player_entity);
            if *user_authority == event.new_user_id() {
                player_cmds.insert(CurrentUser);
            } else {
                player_cmds.remove::<CurrentUser>();
            }
        }
    }
}
