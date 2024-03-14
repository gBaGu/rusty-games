use std::fmt::{Display, Formatter, Write};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TicTacToeError {
    #[error("player id's are the same")]
    SamePlayers,
    #[error("player not found")]
    PlayerNotFound,
    #[error("invalid row (expected: 1-3, found: {0})")]
    InvalidFieldRow(usize),
    #[error("invalid column (expected: 1-3, found: {0})")]
    InvalidFieldCol(usize),
    #[error("cell ({}, {}) is occupied", .0.row, .0.col)]
    CellIsOccupied(FieldCoordinates),
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

#[derive(Clone, Copy, Debug)]
pub struct Player {
    id: PlayerId,
    sign: Sign,
}

impl Player {
    pub fn new(id: PlayerId, sign: Sign) -> Player {
        Self { id, sign }
    }

    pub fn get_id(&self) -> PlayerId {
        self.id
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FinishedState {
    Win(PlayerId),
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameState {
    Turn(PlayerId),
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
pub struct FieldCoordinates {
    row: FieldRow,
    col: FieldCol,
}

impl FieldCoordinates {
    pub fn new(row: FieldRow, col: FieldCol) -> FieldCoordinates {
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
        let p1 = Player::new(id1, Sign::X);
        let p2 = Player::new(id2, Sign::O);
        Ok(Self {
            players: [p1, p2],
            current_player_id: 0,
            state: GameState::Turn(p1.id),
            field: Default::default(), // TODO: switch to Field::empty()
        })
    }

    pub fn get_current_player(&self) -> &Player {
        &self.players[self.current_player_id]
    }

    pub fn get_player_by_sign(&self, sign: Sign) -> Option<&Player> {
        self.players.iter().find(|player| player.sign == sign)
    }

    pub fn make_turn(
        &mut self,
        player: PlayerId,
        coordinates: FieldCoordinates,
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

        self.update_state()?;
        Ok(self.state)
    }

    fn update_state(&mut self) -> Result<(), TicTacToeError> {
        for i in 0..3 {
            let row = self.field[i];
            if let Some(first_sign) = *row.first().unwrap() {
                if row
                    .iter()
                    .all(|sign| matches!(*sign, Some(s) if s == first_sign))
                {
                    return self.set_winner(first_sign);
                }
            }
            if let Some(first_sign) = self.field.first().unwrap()[i] {
                if self
                    .field
                    .iter()
                    .all(|row| matches!(row[i], Some(s) if s == first_sign))
                {
                    return self.set_winner(first_sign);
                }
            }
        }
        // check diagonals
        if let (Some(sign1), Some(sign2), Some(sign3)) =
            (self.field[0][0], self.field[1][1], self.field[2][2])
        {
            if sign1 == sign2 && sign2 == sign3 {
                return self.set_winner(sign1);
            }
        }
        if let (Some(sign1), Some(sign2), Some(sign3)) =
            (self.field[0][2], self.field[1][1], self.field[2][0])
        {
            if sign1 == sign2 && sign2 == sign3 {
                return self.set_winner(sign1);
            }
        }

        if self.field.iter().all(|row| row.iter().all(|s| s.is_some())) {
            self.state = GameState::Finished(FinishedState::Draw);
            return Ok(());
        }

        self.switch_player();
        return Ok(());
    }

    fn set_winner(&mut self, sign: Sign) -> Result<(), TicTacToeError> {
        let player = self.get_player_by_sign(sign).ok_or(TicTacToeError::PlayerNotFound)?;
        self.state = GameState::Finished(FinishedState::Win(player.id));
        return Ok(());
    }

    fn switch_player(&mut self) {
        self.current_player_id = match self.current_player_id {
            0 => 1,
            1 => 0,
            _ => unreachable!(),
        };
        self.state = GameState::Turn(self.get_current_player().id);
    }
}
