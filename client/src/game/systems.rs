use bevy::prelude::*;
use game_server::game::tic_tac_toe::TicTacToe;
use game_server::game::{BoardCell, Game, GameState, PlayerId as GamePlayerId};
use game_server::proto;
use std::ops::Deref;

use super::resources::{LocalGame, RefreshGameTimer};
use super::{
    CellUpdated, CurrentGame, FullGameInfo, GameOver, GameType, LocalGameTurn, NetworkGameTurn,
    Position, StateUpdated, SuccessfulTurn,
};
use crate::commands::CommandsExt;
use crate::grpc::{GrpcClient, RpcResultReady};
use crate::interface::common::ERROR_SOUND_PATH;
use crate::Settings;

fn update_state_on_turn(
    cell_updated: &mut EventWriter<CellUpdated>,
    state_updated: &mut EventWriter<StateUpdated>,
    successful_turn: &mut EventWriter<SuccessfulTurn>,
    game: &mut CurrentGame,
    player_id: GamePlayerId,
    state: GameState,
    pos: Position,
) {
    game.set_cell((pos.row() as usize, pos.col() as usize), player_id);
    game.set_state(state);
    game.clear_pending_move();
    cell_updated.send(CellUpdated::new(pos, player_id));
    state_updated.send(StateUpdated(state));
    successful_turn.send(SuccessfulTurn::new(pos, player_id));
}

/// Emit StateUpdated once the game is initialized
pub fn game_initialized(mut state_updated: EventWriter<StateUpdated>, game: Res<CurrentGame>) {
    state_updated.send(StateUpdated(game.state()));
}

/// Update timer and if it's finished stop it and create a grpc call task
pub fn send_get_game(
    mut commands: Commands,
    mut timer: ResMut<RefreshGameTimer>,
    game: Res<CurrentGame>,
    client: Res<GrpcClient>,
    time: Res<Time>,
) {
    if timer.tick(time.delta()).just_finished() {
        if let GameType::Network(id) = game.game_type() {
            if let Some(task) = client.get_game(id) {
                commands.spawn(task);
                timer.reset();
                timer.pause();
            }
        }
    }
}

pub fn make_turn_network(
    mut commands: Commands,
    mut turn_data: EventReader<NetworkGameTurn>,
    mut game: ResMut<CurrentGame>,
    settings: Res<Settings>,
    client: Res<GrpcClient>,
    asset_server: Res<AssetServer>,
) {
    for event in turn_data.read() {
        if event.auth != game.user_data().auth() {
            println!("{:?} is not authorized to make turn", event.auth);
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            continue;
        }
        let Some(user_id) = settings.user_id() else {
            println!("user id is not set");
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            continue;
        };
        if let Some(task) =
            client.make_turn(event.game_id, user_id, event.pos.row(), event.pos.col())
        {
            game.set_pending_move(event.pos);
            commands.spawn(task);
        } else {
            println!("grpc server is down");
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
        }
    }
}

pub fn make_turn_local(
    mut commands: Commands,
    mut turn_data: EventReader<LocalGameTurn>,
    mut cell_updated: EventWriter<CellUpdated>,
    mut state_updated: EventWriter<StateUpdated>,
    mut successful_turn: EventWriter<SuccessfulTurn>,
    mut game: ResMut<CurrentGame>,
    mut local_game: ResMut<LocalGame>,
    asset_server: Res<AssetServer>,
) {
    for event in turn_data.read() {
        let Some(next_player) = game.get_next_player() else {
            println!("cannot make turn in this game");
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            return;
        };
        if event.auth != next_player.auth() {
            println!("{:?} is not authorized to make turn", event.auth);
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            continue;
        }
        let player_id = next_player.game_player_id();
        let Ok(row) = (event.pos.row() as usize).try_into() else {
            println!("invalid row index: {}", event.pos.row());
            return;
        };
        let Ok(col) = (event.pos.col() as usize).try_into() else {
            println!("invalid row index: {}", event.pos.col());
            return;
        };
        let turn_data = <TicTacToe as Game>::TurnData::new(row, col);
        match local_game.update(player_id, turn_data) {
            Ok(state) => {
                update_state_on_turn(
                    &mut cell_updated,
                    &mut state_updated,
                    &mut successful_turn,
                    &mut game,
                    player_id,
                    state,
                    event.pos,
                );
            }
            Err(err) => {
                println!("failed to update local game: {}", err);
                commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            }
        }
    }
}

pub fn handle_make_turn(
    mut make_turn_result: EventReader<RpcResultReady<proto::MakeTurnReply>>,
    mut commands: Commands,
    mut game: Option<ResMut<CurrentGame>>,
    mut cell_updated: EventWriter<CellUpdated>,
    mut state_updated: EventWriter<StateUpdated>,
    mut successful_turn: EventWriter<SuccessfulTurn>,
    asset_server: Res<AssetServer>,
) {
    for event in make_turn_result.read() {
        let Some(game) = &mut game else {
            println!("no current game, dropping MakeTurn response");
            continue;
        };
        let Some(pending_move) = game.pending_move() else {
            println!("no pending move, dropping MakeTurn response");
            continue;
        };
        let player_id = game.user_data().game_player_id();
        if !matches!(game.get_next_player(), Some(player) if player.game_player_id() == player_id) {
            println!("not your turn");
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            continue;
        }
        match event.deref() {
            Ok(response) => {
                if let Some(new_state) = &response.get_ref().game_state {
                    match new_state.clone().try_into() {
                        Ok(state) => {
                            update_state_on_turn(
                                &mut cell_updated,
                                &mut state_updated,
                                &mut successful_turn,
                                game,
                                player_id,
                                state,
                                pending_move,
                            );
                        }
                        Err(err) => {
                            println!("failed to convert game state: {}", err);
                        }
                    }
                } else {
                    println!("MakeTurn returned empty response");
                    commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                }
            }
            Err(err) => {
                println!("MakeTurn request failed: {}", err);
                commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            }
        };
    }
}

/// Read grpc call result event, unpause timer and update game
pub fn handle_get_game(
    mut get_game_result: EventReader<RpcResultReady<proto::GetGameReply>>,
    mut game: ResMut<CurrentGame>,
    mut timer: ResMut<RefreshGameTimer>,
    mut cell_updated: EventWriter<CellUpdated>,
    mut state_updated: EventWriter<StateUpdated>,
) {
    for event in get_game_result.read() {
        timer.unpause();
        match event.deref() {
            Ok(response) => {
                let Some(info) = &response.get_ref().game_info else {
                    println!("dropping empty GetGame response");
                    continue;
                };
                let GameType::Network(game_id) = game.game_type() else {
                    println!("dropping GetGame response for local game");
                    continue;
                };
                if info.game_id != game_id {
                    println!("received game info for other game, dropping");
                    continue;
                }
                let full_info = match FullGameInfo::try_from(info.clone()) {
                    Ok(info) => info,
                    Err(err) => {
                        println!("failed to decode full game info: {}", err);
                        continue;
                    }
                };
                for (i, row) in full_info.board.iter().enumerate() {
                    for (j, cell) in row.iter().enumerate() {
                        if let BoardCell(Some(player)) = cell {
                            let local_cell = game.board()[(i, j).into()];
                            if local_cell.is_none() {
                                game.set_cell((i, j), *player);
                                let pos = Position::new(i as u32, j as u32);
                                cell_updated.send(CellUpdated::new(pos, *player));
                            }
                        }
                    }
                }
                if full_info.info.state != game.state() {
                    game.set_state(full_info.info.state);
                    state_updated.send(StateUpdated(full_info.info.state));
                }
            }
            Err(err) => {
                println!("GetGame request failed: {}", err);
            }
        }
    }
}

pub fn handle_state_updated(
    mut state_updated: EventReader<StateUpdated>,
    mut game_over: EventWriter<GameOver>,
) {
    if let Some(event) = state_updated.read().last() {
        println!("received StateUpdated: {:?}", event);
        match event.0 {
            GameState::Turn(_) => {}
            GameState::Finished(f) => {
                game_over.send(GameOver(f));
            }
        };
    }
}
