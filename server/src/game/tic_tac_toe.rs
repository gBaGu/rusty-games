use generic_array::typenum;
use prost::Message;

use super::encoding::{FromProtobuf, FromProtobufError};
use super::grid::{Grid, GridIndex};
use crate::game::error::GameError;
use crate::game::player_pool::PlayerIdQueue;
use crate::game::{BoardCell, Game, GameResult, GameState, PlayerId};
use crate::proto::Position;

pub fn winning_combinations() -> [(GridIndex, GridIndex, GridIndex); 8] {
    [
        ((0, 0).into(), (0, 1).into(), (0, 2).into()),
        //     GridIndex::new(0, 0),
        //     GridIndex::new(0, 1),
        //     GridIndex::new(0, 2),
        // ),
        (
            GridIndex::new(1, 0),
            GridIndex::new(1, 1),
            GridIndex::new(1, 2),
        ),
        (
            GridIndex::new(2, 0),
            GridIndex::new(2, 1),
            GridIndex::new(2, 2),
        ),
        (
            GridIndex::new(0, 0),
            GridIndex::new(1, 0),
            GridIndex::new(2, 0),
        ),
        (
            GridIndex::new(0, 1),
            GridIndex::new(1, 1),
            GridIndex::new(2, 1),
        ),
        (
            GridIndex::new(0, 2),
            GridIndex::new(1, 2),
            GridIndex::new(2, 2),
        ),
        (
            GridIndex::new(0, 0),
            GridIndex::new(1, 1),
            GridIndex::new(2, 2),
        ),
        (
            GridIndex::new(2, 0),
            GridIndex::new(1, 1),
            GridIndex::new(0, 2),
        ),
    ]
}

type Cell = BoardCell<PlayerId>;

impl FromProtobuf for GridIndex {
    fn from_protobuf(buf: &[u8]) -> Result<Self, FromProtobufError> {
        let pos = Position::decode(buf)?;
        let row: usize = usize::try_from(pos.row)?;
        let col: usize = usize::try_from(pos.col)?;
        Ok(Self::new(row, col))
    }
}

#[derive(Debug)]
pub struct TicTacToe {
    players: PlayerIdQueue<PlayerId>,
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
    type Players = PlayerIdQueue<PlayerId>;
    type Board = Grid<Cell, typenum::U3, typenum::U3>;

    fn new() -> Self {
        Self::default()
    }

    fn update(&mut self, id: PlayerId, data: Self::TurnData) -> GameResult<GameState> {
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
