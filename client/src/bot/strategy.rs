use game_server::game::tic_tac_toe::{FieldCol, FieldRow, TicTacToe};
use game_server::game::{BoardCell, Game, PlayerId as GamePlayerId};
use rand::{thread_rng, Rng};
use tic_tac_toe_ai::Agent;

use crate::game::{Position, BOARD_SIZE};

fn get_random_position(board: &[[BoardCell<GamePlayerId>; BOARD_SIZE]]) -> Position {
    let mut rng = thread_rng();
    let empty_cells: Vec<_> = board
        .iter()
        .enumerate()
        .map(|(i, row)| row.iter().enumerate().map(move |(j, cell)| (i, j, cell)))
        .flatten()
        .filter(|(_, _, cell)| cell.is_none())
        .collect();
    let index = rng.gen_range(0..empty_cells.len());
    let rand_cell = empty_cells[index];
    Position::new(rand_cell.0 as u32, rand_cell.1 as u32)
}

pub enum MoveStrategy {
    Random,
    QLearningModel(Agent),
}

impl MoveStrategy {
    pub fn get_move(&self, board: &[[BoardCell<GamePlayerId>; BOARD_SIZE]]) -> Option<Position> {
        match self {
            MoveStrategy::Random => Some(get_random_position(board)),
            MoveStrategy::QLearningModel(agent) => {
                // TODO: use this board in client
                let mut ttt_board = <TicTacToe as Game>::Board::default();
                for (i, row) in board.iter().enumerate() {
                    for (j, cell) in row.iter().enumerate() {
                        *ttt_board.get_mut_ref(<TicTacToe as Game>::TurnData::new(
                            FieldRow::try_from(i).ok()?,
                            FieldCol::try_from(j).ok()?,
                        )) = *cell;
                    }
                }
                agent
                    .get_action(&ttt_board)
                    .and_then(|action| Some(Position::new(action.0 as u32, action.1 as u32)))
            }
        }
    }
}
