use std::ops::Deref;

use bevy::prelude::*;
use game_server::game::grid::GridIndex;
use game_server::game::tic_tac_toe::TicTacToe;
use game_server::game::{BoardCell, FinishedState, Game, GameState, PlayerId as PlayerPosition};
use game_server::proto;

use super::resources::{LocalGame, RefreshGameTimer};
use super::{
    Authority, CellUpdated, CurrentGame, FullGameInfo, GameOver, GameType, LocalGameTurn,
    NetworkGameTurn, PlayerWon, Position, StateUpdated, SuccessfulTurn,
};
use crate::board::{TileFilled, TilePressed};
use crate::commands::CommandsExt;
use crate::grpc::{GrpcClient, RpcResultReady};
use crate::interface::common::ERROR_SOUND_PATH;
use crate::Settings;

fn update_state_on_turn(
    cell_updated: &mut EventWriter<CellUpdated>,
    state_updated: &mut EventWriter<StateUpdated>,
    successful_turn: &mut EventWriter<SuccessfulTurn>,
    game: &mut CurrentGame,
    player_position: PlayerPosition,
    state: GameState,
    pos: Position,
) {
    game.set_cell(pos, player_position);
    game.set_state(state);
    game.clear_pending_move();
    cell_updated.send(CellUpdated::new(pos, player_position));
    state_updated.send(StateUpdated(state));
    successful_turn.send(SuccessfulTurn::new(player_position));
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
            match client.get_game(proto::GameType::TicTacToe, id) {
                Ok(task) => {
                    commands.spawn(task);
                    timer.reset();
                    timer.pause();
                }
                Err(err) => println!("get_game call failed: {}", err),
            }
        }
    }
}

/// Tracks ButtonPressed event from board and transforms it into NetworkGameTurn/LocalGameTurn event
pub fn make_turn(
    mut commands: Commands,
    mut board_button_pressed: EventReader<TilePressed>,
    mut network_turn_data: EventWriter<NetworkGameTurn>,
    mut local_turn_data: EventWriter<LocalGameTurn>,
    game: Res<CurrentGame>,
    settings: Res<Settings>,
    asset_server: Res<AssetServer>,
) {
    let Some(user_id) = settings.user_id() else {
        println!("user id is not set");
        return;
    };
    for event in board_button_pressed.read() {
        if game.board()[event.pos().into()].is_some() {
            println!("cell is occupied");
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            continue;
        }
        if matches!(game.state(), GameState::Finished(_)) {
            println!("cannot make turn in finished game");
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            continue;
        }
        game.trigger_turn(
            &mut network_turn_data,
            &mut local_turn_data,
            Authority::Player(user_id),
            event.pos(),
        );
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
        match client.make_turn(
            proto::GameType::TicTacToe,
            event.game_id,
            user_id,
            GridIndex::from(event.pos),
        ) {
            Ok(task) => {
                game.set_pending_move(event.pos);
                commands.spawn(task);
            }
            Err(err) => {
                println!("make_turn call failed: {}", err);
                commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            }
        };
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
        let player_id = next_player.player_position();
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
        let player_id = game.user_data().player_position();
        if !matches!(game.get_next_player(), Some(player) if player.player_position() == player_id)
        {
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
                            let pos = Position::new(i, j);
                            let local_cell = game.board()[pos.into()];
                            if local_cell.is_none() {
                                game.set_cell(pos, *player);
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
    mut player_won: EventWriter<PlayerWon>,
    mut game_over: EventWriter<GameOver>,
    game: Res<CurrentGame>,
) {
    if let Some(event) = state_updated.read().last() {
        println!("received StateUpdated: {:?}", event);
        match event.0 {
            GameState::Turn(_) => {}
            GameState::Finished(f) => match f {
                FinishedState::Win(player) => {
                    if let Some(data) = game.get_player_data(player) {
                        player_won.send(PlayerWon(data.auth()));
                    } else {
                        println!("couldn't get winner ({}) data", player);
                    }
                }
                FinishedState::Draw => {
                    game_over.send(GameOver::new(None));
                }
            },
        };
    }
}

pub fn handle_cell_updated(
    mut cell_updated: EventReader<CellUpdated>,
    mut tile_filled: EventWriter<TileFilled>,
    game: Res<CurrentGame>,
) {
    for event in cell_updated.read() {
        let Some(img) = game.get_player_image(event.player_position()) else {
            println!("failed to get player image, dropping event");
            continue;
        };
        let Some(board) = game.board_entity() else {
            println!("ui board is not set for this game, dropping event");
            continue;
        };
        tile_filled.send(TileFilled::new(*board, event.pos(), img.clone()));
    }
}
