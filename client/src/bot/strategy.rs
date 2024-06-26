use rand::{thread_rng, Rng};
use tic_tac_toe_ai::Agent;

use crate::game::{Position, TTTBoard};

fn get_random_position(board: &TTTBoard) -> Position {
    let mut rng = thread_rng();
    let empty_cells: Vec<_> = board
        .all_indexed()
        .filter(|(_, cell)| cell.is_none())
        .collect();
    let index = rng.gen_range(0..empty_cells.len());
    let rand_cell = empty_cells[index];
    Position::new(rand_cell.0.row() as u32, rand_cell.0.col() as u32)
}

pub enum MoveStrategy {
    Random,
    QLearningModel(Agent),
}

impl MoveStrategy {
    pub fn get_move(&self, board: &TTTBoard) -> Option<Position> {
        match self {
            MoveStrategy::Random => Some(get_random_position(board)),
            MoveStrategy::QLearningModel(agent) => agent
                .get_best_action(&board)
                .and_then(|action| Some(Position::new(action.0 as u32, action.1 as u32))),
        }
    }
}
