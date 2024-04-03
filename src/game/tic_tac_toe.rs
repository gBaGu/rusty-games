use generic_array::typenum::Unsigned;
use std::fmt::{Display, Formatter, Write};

use crate::game::{
    error::GameError,
    grid::{Grid, GridIndex, WithMaxValue},
    player_pool::{PlayerId, PlayerPool, WithPlayerId},
};

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

#[derive(Clone, Copy, Debug)]
pub struct Player {
    id: PlayerId,
    sign: Sign,
}

impl Player {
    pub fn new(id: PlayerId, sign: Sign) -> Player {
        Self { id, sign }
    }
}

impl WithPlayerId for Player {
    fn get_id(&self) -> PlayerId {
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

// TODO: use derive macro instead
impl WithMaxValue for FieldRow {
    type MaxValue = generic_array::typenum::U3;
}

impl TryFrom<usize> for FieldRow {
    type Error = GameError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::R1),
            1 => Ok(Self::R2),
            2 => Ok(Self::R3),
            _ => Err(Self::Error::InvalidGridRow {
                max_expected: <Self as WithMaxValue>::MaxValue::to_usize(),
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldCol {
    C1,
    C2,
    C3,
}

// TODO: use derive macro instead
impl WithMaxValue for FieldCol {
    type MaxValue = generic_array::typenum::U3;
}

impl TryFrom<usize> for FieldCol {
    type Error = GameError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::C1),
            1 => Ok(Self::C2),
            2 => Ok(Self::C3),
            _ => Err(Self::Error::InvalidGridCol {
                max_expected: <Self as WithMaxValue>::MaxValue::to_usize(),
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

#[derive(Debug)]
pub struct TicTacToe {
    players: PlayerPool<Player>,
    state: GameState,
    field: Grid<Option<Sign>, FieldRow, FieldCol>,
}

impl TicTacToe {
    pub fn new(id1: PlayerId, id2: PlayerId) -> Result<Self, GameError> {
        if id1 == id2 {
            return Err(GameError::DuplicatePlayerId);
        }
        let p1 = Player::new(id1, Sign::X);
        let p2 = Player::new(id2, Sign::O);
        Ok(Self {
            players: PlayerPool::new([p1, p2].to_vec()),
            state: GameState::Turn(p1.id),
            field: Grid::empty(),
        })
    }

    pub fn get_current_player(&mut self) -> Result<&Player, GameError> {
        self.players
            .get_current()
            .ok_or(GameError::PlayerPoolCorrupted)
    }

    pub fn get_player_by_sign(&self, sign: Sign) -> Option<&Player> {
        self.players.find(|player| player.sign == sign)
    }

    pub fn get_state(&self) -> &GameState {
        &self.state
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.state, GameState::Finished(_))
    }

    pub fn make_turn(
        &mut self,
        player: PlayerId,
        coordinates: GridIndex<FieldRow, FieldCol>,
    ) -> Result<GameState, GameError> {
        if matches!(self.state, GameState::Finished(_)) {
            return Err(GameError::GameIsFinished);
        }
        if player != self.get_current_player()?.id {
            return Err(GameError::NotYourTurn {
                expected: self.get_current_player()?.id,
                found: player,
            });
        }

        let sign = self.get_current_player()?.sign;
        let cell = self.get_cell(coordinates);
        if cell.is_some() {
            return Err(GameError::CellIsOccupied {
                row: coordinates.get_row(),
                col: coordinates.get_col(),
            });
        }
        *cell = Some(sign);

        self.update_state()
    }

    fn get_cell(&mut self, coordinates: GridIndex<FieldRow, FieldCol>) -> &mut Option<Sign> {
        self.field.get_mut_ref(coordinates)
    }

    fn update_state(&mut self) -> Result<GameState, GameError> {
        for i in 0..3 {
            let row = &self.field[i];
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
            return Ok(self.state);
        }

        self.switch_player()
    }

    fn set_winner(&mut self, sign: Sign) -> Result<GameState, GameError> {
        let player = self
            .get_player_by_sign(sign)
            .ok_or(GameError::PlayerNotFound)?;
        self.state = GameState::Finished(FinishedState::Win(player.id));
        Ok(self.state)
    }

    fn switch_player(&mut self) -> Result<GameState, GameError> {
        let next_player = self.players.next().ok_or(GameError::PlayerPoolCorrupted)?;
        self.state = GameState::Turn(next_player.id);
        Ok(self.state)
    }
}
