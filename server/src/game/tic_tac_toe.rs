use generic_array::typenum::Unsigned;
use prost::Message;

use crate::game::encoding::{FromProtobuf, FromProtobufError};
use crate::game::game::{BoardCell, Game, GameResult, GameState};
use crate::game::{
    error::GameError,
    grid::{Grid, GridIndex, WithLength},
    player_pool::{PlayerId, PlayerPool, WithPlayerId},
    tic_tac_toe,
};
use crate::proto::Position;

#[derive(Clone, Copy, Debug)]
pub struct Player {
    id: PlayerId,
}

impl Player {
    pub fn new(id: PlayerId) -> Player {
        Self { id }
    }
}

impl WithPlayerId for Player {
    fn get_id(&self) -> PlayerId {
        self.id
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FieldRow {
    R1,
    R2,
    R3,
}

// TODO: use derive macro instead
impl WithLength for FieldRow {
    type Length = generic_array::typenum::U3;
}

impl TryFrom<usize> for FieldRow {
    type Error = FromProtobufError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::R1),
            1 => Ok(Self::R2),
            2 => Ok(Self::R3),
            _ => Err(Self::Error::InvalidGridRow {
                max_expected: <Self as WithLength>::Length::to_usize() - 1,
                found: value,
            }),
        }
    }
}

impl From<FieldRow> for usize {
    fn from(value: FieldRow) -> Self {
        match value {
            FieldRow::R1 => 0,
            FieldRow::R2 => 1,
            FieldRow::R3 => 2,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FieldCol {
    C1,
    C2,
    C3,
}

// TODO: use derive macro instead
impl WithLength for FieldCol {
    type Length = generic_array::typenum::U3;
}

impl TryFrom<usize> for FieldCol {
    type Error = FromProtobufError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::C1),
            1 => Ok(Self::C2),
            2 => Ok(Self::C3),
            _ => Err(Self::Error::InvalidGridCol {
                max_expected: <Self as WithLength>::Length::to_usize() - 1,
                found: value,
            }),
        }
    }
}

impl From<FieldCol> for usize {
    fn from(value: FieldCol) -> Self {
        match value {
            FieldCol::C1 => 0,
            FieldCol::C2 => 1,
            FieldCol::C3 => 2,
        }
    }
}

fn winning_combinations() -> [(Index, Index, Index); 8] {
    [
        (
            Index::new(FieldRow::R1, FieldCol::C1),
            Index::new(FieldRow::R1, FieldCol::C2),
            Index::new(FieldRow::R1, FieldCol::C3),
        ),
        (
            Index::new(FieldRow::R2, FieldCol::C1),
            Index::new(FieldRow::R2, FieldCol::C2),
            Index::new(FieldRow::R2, FieldCol::C3),
        ),
        (
            Index::new(FieldRow::R3, FieldCol::C1),
            Index::new(FieldRow::R3, FieldCol::C2),
            Index::new(FieldRow::R3, FieldCol::C3),
        ),
        (
            Index::new(FieldRow::R1, FieldCol::C1),
            Index::new(FieldRow::R2, FieldCol::C1),
            Index::new(FieldRow::R3, FieldCol::C1),
        ),
        (
            Index::new(FieldRow::R1, FieldCol::C2),
            Index::new(FieldRow::R2, FieldCol::C2),
            Index::new(FieldRow::R3, FieldCol::C2),
        ),
        (
            Index::new(FieldRow::R1, FieldCol::C3),
            Index::new(FieldRow::R2, FieldCol::C3),
            Index::new(FieldRow::R3, FieldCol::C3),
        ),
        (
            Index::new(FieldRow::R1, FieldCol::C1),
            Index::new(FieldRow::R2, FieldCol::C2),
            Index::new(FieldRow::R3, FieldCol::C3),
        ),
        (
            Index::new(FieldRow::R3, FieldCol::C1),
            Index::new(FieldRow::R2, FieldCol::C2),
            Index::new(FieldRow::R1, FieldCol::C3),
        ),
    ]
}

type Cell = BoardCell<PlayerId>;

#[derive(Debug)]
pub struct TicTacToe {
    players: PlayerPool<Player>,
    state: GameState,
    field: Grid<Cell, FieldRow, FieldCol>,
}

type Index = GridIndex<FieldRow, FieldCol>;

impl FromProtobuf for Index {
    fn from_protobuf(buf: &[u8]) -> Result<Self, FromProtobufError> {
        let pos = Position::decode(buf)?;
        let row: usize = usize::try_from(pos.row)?;
        let col: usize = usize::try_from(pos.col)?;
        let row = tic_tac_toe::FieldRow::try_from(row)?;
        let col = tic_tac_toe::FieldCol::try_from(col)?;
        Ok(Self::new(row, col))
    }
}

impl Game for TicTacToe {
    type TurnData = Index;
    type Players = PlayerPool<Player>;
    type Board = Grid<Cell, FieldRow, FieldCol>;

    fn new(players: &[PlayerId]) -> GameResult<Self> {
        let [id1, id2]: [_; 2] = players
            .try_into()
            .map_err(|_| GameError::invalid_players_number(2, players.len()))?;
        if id1 == id2 {
            return Err(GameError::DuplicatePlayerId);
        }
        let p1 = Player::new(id1);
        let p2 = Player::new(id2);
        Ok(Self {
            players: Self::Players::new([p1, p2].to_vec()),
            state: GameState::Turn(p1.id),
            field: Grid::default(),
        })
    }

    fn update(&mut self, id: PlayerId, data: Self::TurnData) -> GameResult<GameState> {
        if matches!(self.state, GameState::Finished(_)) {
            return Err(GameError::GameIsFinished);
        }
        if id != self.get_current_player()?.id {
            return Err(GameError::not_your_turn(self.get_current_player()?.id, id));
        }

        let player_id = self.get_current_player()?.id;
        let cell = self.get_cell_mut(data);
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
    fn get_cell(&self, position: Index) -> &Cell {
        self.field.get_ref(position)
    }

    fn get_cell_mut(&mut self, position: Index) -> &mut Cell {
        self.field.get_mut_ref(position)
    }

    fn update_state(&mut self) -> GameResult<GameState> {
        for (idx1, idx2, idx3) in winning_combinations() {
            if let (BoardCell(Some(p1)), BoardCell(Some(p2)), BoardCell(Some(p3))) = (
                self.get_cell(idx1),
                self.get_cell(idx2),
                self.get_cell(idx3),
            ) {
                if p1 == p2 && p2 == p3 {
                    return Ok(self.set_winner(*p1));
                }
            }
        }

        if self.field.iter().flatten().all(|cell| cell.is_some()) {
            return Ok(self.set_draw());
        }

        self.switch_player()
    }
}
