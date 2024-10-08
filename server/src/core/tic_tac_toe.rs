use generic_array::typenum;

use super::grid::{Grid, GridIndex};
use super::player_pool::PlayerIdQueue;
use crate::core::{BoardCell, Game, GameError, GameResult, GameState, PlayerPosition};

pub fn winning_combinations() -> [(GridIndex, GridIndex, GridIndex); 8] {
    [
        ((0, 0).into(), (0, 1).into(), (0, 2).into()),
        ((1, 0).into(), (1, 1).into(), (1, 2).into()),
        ((2, 0).into(), (2, 1).into(), (2, 2).into()),
        ((0, 0).into(), (1, 0).into(), (2, 0).into()),
        ((0, 1).into(), (1, 1).into(), (2, 1).into()),
        ((0, 2).into(), (1, 2).into(), (2, 2).into()),
        ((0, 0).into(), (1, 1).into(), (2, 2).into()),
        ((2, 0).into(), (1, 1).into(), (0, 2).into()),
    ]
}

type Cell = BoardCell<PlayerPosition>;

#[derive(Clone, Debug)]
pub struct TicTacToe {
    players: PlayerIdQueue<PlayerPosition>,
    state: GameState,
    field: Grid<Cell, typenum::U3, typenum::U3>,
}

impl Default for TicTacToe {
    fn default() -> Self {
        let players = (0..Self::NUM_PLAYERS).map(|id| id.into()).collect();
        Self {
            players: PlayerIdQueue::new(players),
            state: GameState::Turn(0),
            field: Grid::default(),
        }
    }
}

impl Game for TicTacToe {
    const NUM_PLAYERS: u8 = 2;
    type TurnData = GridIndex;
    type Players = PlayerIdQueue<PlayerPosition>;
    type Board = Grid<Cell, typenum::U3, typenum::U3>;

    fn new() -> Self {
        Self::default()
    }

    fn update(&mut self, id: PlayerPosition, data: Self::TurnData) -> GameResult<GameState> {
        if matches!(self.state, GameState::Finished(_)) {
            return Err(GameError::GameIsFinished);
        }
        if id != *self.get_current_player()? {
            return Err(GameError::not_your_turn(*self.get_current_player()?, id));
        }

        let player_id = *self.get_current_player()?;
        let cell = &mut self.field[data];
        if cell.is_some() {
            return Err(GameError::cell_is_occupied(
                data.row().into(),
                data.col().into(),
            ));
        }
        *cell = player_id.into();

        self.update_state()
    }

    fn board(&self) -> &Self::Board {
        &self.field
    }

    fn board_mut(&mut self) -> &mut Self::Board {
        &mut self.field
    }

    fn players(&self) -> &Self::Players {
        &self.players
    }

    fn players_mut(&mut self) -> &mut Self::Players {
        &mut self.players
    }

    fn state(&self) -> GameState {
        self.state
    }

    fn set_state(&mut self, state: GameState) {
        self.state = state;
    }

    fn set_board(&mut self, board: Self::Board) {
        self.field = board;
    }
}

impl TicTacToe {
    fn update_state(&mut self) -> GameResult<GameState> {
        for (idx1, idx2, idx3) in winning_combinations() {
            if let (BoardCell(Some(p1)), BoardCell(Some(p2)), BoardCell(Some(p3))) =
                (self.field[idx1], self.field[idx2], self.field[idx3])
            {
                if p1 == p2 && p2 == p3 {
                    return Ok(self.set_winner(p1));
                }
            }
        }

        if self.field.iter().flatten().all(|cell| cell.is_some()) {
            return Ok(self.set_draw());
        }

        self.switch_player()
    }
}
