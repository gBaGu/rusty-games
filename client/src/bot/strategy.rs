use rand::{Rng, thread_rng};
use game_server::game::{BoardCell, PlayerId as GamePlayerId};

use crate::game::{BOARD_SIZE, Position};

fn get_random_position(board: &[[BoardCell<GamePlayerId>; BOARD_SIZE]]) -> Position {
    let mut rng = thread_rng();
    let empty_cells: Vec<_> = board.iter()
        .enumerate()
        .map(|(i, row)| row.iter().enumerate().map(move |(j, cell)| (i, j, cell)))
        .flatten()
        .filter(|(_, _, cell)| cell.is_none())
        .collect();
    let index = rng.gen_range(0..empty_cells.len());
    let rand_cell = empty_cells[index];
    Position::new(rand_cell.0 as u32, rand_cell.1 as u32)
}

#[derive(Debug)]
pub enum MoveStrategy {
    Random,
}

impl MoveStrategy {
    pub fn get_move(&self, board: &[[BoardCell<GamePlayerId>; BOARD_SIZE]]) -> Position {
        match self {
            MoveStrategy::Random => {
                get_random_position(board)
            }
        }
    }
}