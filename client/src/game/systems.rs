use bevy::prelude::*;
use game_server::game::{BoardCell, GameState};

use super::resources::RefreshGameTimer;
use super::{
    CellUpdated, CurrentGame, FullGameInfo, GameOver, Position, StateUpdated, SuccessfulTurn,
};
use crate::commands::CommandsExt;
use crate::grpc::{CallGetGame, CallMakeTurn, GrpcClient, TaskEntity};
use crate::interface::common::ERROR_SOUND_PATH;

/// Emit StateUpdated once the game is initialized
pub fn game_initialized(mut state_updated: EventWriter<StateUpdated>, game: Res<CurrentGame>) {
    state_updated.send(StateUpdated(game.state()));
}

pub fn refresh_game(
    mut commands: Commands,
    mut timer: ResMut<RefreshGameTimer>,
    game: Res<CurrentGame>,
    client: ResMut<GrpcClient>,
    time: Res<Time>,
) {
    if timer.tick(time.delta()).just_finished() {
        if let Some(task) = client.load_game(game.id()) {
            commands.spawn(CallGetGame(task));
        }
    }
}

pub fn handle_turn(
    mut make_turn: Query<(Entity, &mut CallMakeTurn, &Position)>,
    mut commands: Commands,
    mut game: Option<ResMut<CurrentGame>>,
    mut cell_updated: EventWriter<CellUpdated>,
    mut state_updated: EventWriter<StateUpdated>,
    mut successful_turn: EventWriter<SuccessfulTurn>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut task, pos) in make_turn.iter_mut() {
        let mut task = TaskEntity::new(commands.reborrow(), entity, &mut task.0);
        if let Some(res) = task.poll_once() {
            let Some(game) = &mut game else {
                println!("no current game, dropping MakeTurn response");
                continue;
            };
            let user_id = game.user_id();
            if !matches!(game.get_next_player(), Some(id) if id == user_id) {
                println!("not your turn");
                commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                continue;
            }
            match res {
                Ok(response) => {
                    if let Some(new_state) = response.into_inner().game_state {
                        match new_state.try_into() {
                            Ok(state) => {
                                game.set_cell((pos.row() as usize, pos.col() as usize), user_id);
                                game.set_state(state);
                                cell_updated.send(CellUpdated::new(*pos, user_id));
                                state_updated.send(StateUpdated(state));
                                successful_turn.send(SuccessfulTurn::new(*pos, user_id));
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
}

pub fn update_game(
    mut get_game: Query<(Entity, &mut CallGetGame)>,
    mut commands: Commands,
    mut game: Option<ResMut<CurrentGame>>,
    mut cell_updated: EventWriter<CellUpdated>,
    mut state_updated: EventWriter<StateUpdated>,
) {
    for (entity, mut task) in get_game.iter_mut() {
        let mut task = TaskEntity::new(commands.reborrow(), entity, &mut task.0);
        if let Some(res) = task.poll_once() {
            let Some(game) = &mut game else {
                println!("no current game, dropping GetGame response");
                continue;
            };
            match res {
                Ok(response) => {
                    let Some(info) = response.into_inner().game_info else {
                        println!("dropping empty GetGame response");
                        continue;
                    };
                    if info.game_id != game.id() {
                        println!("received game info for other game, dropping");
                        continue;
                    }
                    let full_info = match FullGameInfo::try_from(info) {
                        Ok(info) => info,
                        Err(err) => {
                            println!("failed to decode full game info: {}", err);
                            continue;
                        }
                    };
                    for (i, row) in full_info.board.iter().enumerate() {
                        for (j, cell) in row.iter().enumerate() {
                            if let BoardCell(Some(player)) = cell {
                                let local_cell = game.board()[i][j];
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
