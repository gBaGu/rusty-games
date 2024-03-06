use std::fmt::{Display, Formatter, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TicTacToeError {
    #[error("player id's are the same")]
    SamePlayers,
    #[error("invalid row (expected: 1-3, found: {0})")]
    InvalidFieldRow(usize),
    #[error("invalid column (expected: 1-3, found: {0})")]
    InvalidFieldCol(usize),
    #[error("cell ({}, {}) is occupied", .0.row, .0.col)]
    CellIsOccupied(TurnData),
    #[error("can't make turn on a finished game")]
    GameIsFinished,
    #[error("other player's turn (expected: {expected}, found: {found}")]
    NotYourTurn { expected: PlayerId, found: PlayerId },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sign {
    X,
    O,
}

impl From<Sign> for char {
    fn from(value: Sign) -> Self {
        match value {
            Sign::O => 'O',
            Sign::X => 'X',
        }
    }
}

impl Display for Sign {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char((*self).into())
    }
}

pub type PlayerId = u64;

#[derive(Clone, Debug)]
pub struct Player {
    id: PlayerId,
    sign: Sign,
}

impl Player {
    pub fn new(id: PlayerId, sign: Sign) -> Player {
        Self { id, sign }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FinishedState {
    Win(Sign),
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameState {
    NotStarted,
    Started,
    Finished(FinishedState),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldRow {
    R1,
    R2,
    R3,
}

impl Display for FieldRow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", <FieldRow as Into<usize>>::into(*self))
    }
}

impl TryFrom<usize> for FieldRow {
    type Error = TicTacToeError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::R1),
            2 => Ok(Self::R2),
            3 => Ok(Self::R3),
            _ => Err(Self::Error::InvalidFieldRow(value)),
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldCol {
    C1,
    C2,
    C3,
}

impl Display for FieldCol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", <FieldCol as Into<usize>>::into(*self))
    }
}

impl TryFrom<usize> for FieldCol {
    type Error = TicTacToeError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::C1),
            2 => Ok(Self::C2),
            3 => Ok(Self::C3),
            _ => Err(Self::Error::InvalidFieldCol(value)),
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurnData {
    row: FieldRow,
    col: FieldCol,
}

impl TurnData {
    pub fn new(row: FieldRow, col: FieldCol) -> TurnData {
        Self { row, col }
    }
}

#[derive(Debug)]
pub struct TicTacToe {
    players: [Player; 2], // TODO: move to PlayerPool to separate switch turn logic
    current_player_id: usize,
    state: GameState,
    field: [[Option<Sign>; 3]; 3], // TODO: move to Field to hide arrays and indexing
}

impl TicTacToe {
    pub fn new(id1: PlayerId, id2: PlayerId) -> Result<Self, TicTacToeError> {
        if id1 == id2 {
            return Err(TicTacToeError::SamePlayers);
        }
        Ok(Self {
            players: [Player::new(id1, Sign::X), Player::new(id2, Sign::O)],
            current_player_id: 0,
            state: GameState::NotStarted,
            field: Default::default(), // TODO: switch to Field::empty()
        })
    }

    pub fn get_current_player(&self) -> &Player {
        &self.players[self.current_player_id]
    }

    pub fn make_turn(
        &mut self,
        player: PlayerId,
        coordinates: TurnData,
    ) -> Result<GameState, TicTacToeError> {
        if matches!(self.state, GameState::Finished(_)) {
            return Err(TicTacToeError::GameIsFinished);
        }
        if player != self.get_current_player().id {
            return Err(TicTacToeError::NotYourTurn {
                expected: self.get_current_player().id,
                found: player,
            });
        }

        let row: usize = coordinates.row.into();
        let col: usize = coordinates.col.into();
        let sign = self.get_current_player().sign;
        let cell = self.field.get_mut(row).unwrap().get_mut(col).unwrap();
        if cell.is_some() {
            return Err(TicTacToeError::CellIsOccupied(coordinates));
        }
        *cell = Some(sign);

        if self.state == GameState::NotStarted {
            self.state = GameState::Started;
        } else if let Some(finished) = self.check_if_finished() {
            self.state = GameState::Finished(finished);
        }
        self.switch_player();
        Ok(self.state)
    }

    fn switch_player(&mut self) {
        self.current_player_id = match self.current_player_id {
            0 => 1,
            1 => 0,
            _ => unreachable!(),
        }
    }

    fn check_if_finished(&self) -> Option<FinishedState> {
        for i in 0..3 {
            let row = self.field[i];
            if let Some(first_sign) = *row.first().unwrap() {
                if row
                    .iter()
                    .all(|sign| matches!(*sign, Some(s) if s == first_sign))
                {
                    return Some(FinishedState::Win(first_sign));
                }
            }
            if let Some(first_sign) = self.field.first().unwrap()[i] {
                if self
                    .field
                    .iter()
                    .all(|row| matches!(row[i], Some(s) if s == first_sign))
                {
                    return Some(FinishedState::Win(first_sign));
                }
            }
        }
        if let (Some(e1), Some(e2), Some(e3)) =
            (self.field[0][0], self.field[1][1], self.field[2][2])
        {
            if e1 == e2 && e2 == e3 {
                return Some(FinishedState::Win(e1));
            }
        }
        if let (Some(e1), Some(e2), Some(e3)) =
            (self.field[0][2], self.field[1][1], self.field[2][0])
        {
            if e1 == e2 && e2 == e3 {
                return Some(FinishedState::Win(e1));
            }
        }

        if self.field.iter().all(|row| row.iter().all(|s| s.is_some())) {
            return Some(FinishedState::Draw);
        }
        None
    }
}
