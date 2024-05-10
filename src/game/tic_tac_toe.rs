use generic_array::typenum::Unsigned;
use prost::Message;

use crate::game::game::{FromProtobuf, FromProtobufError, Game, GameResult, GameState};
use crate::game::{
    error::GameError,
    grid::{Grid, GridIndex, WithLength},
    player_pool::{PlayerId, PlayerPool, PlayerQueue, WithPlayerId},
    tic_tac_toe,
};
use crate::proto::Position;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sign {
    X,
    O,
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

type Cell = Option<Sign>;

#[derive(Debug)]
pub struct TicTacToe {
    players: PlayerPool<Player>,
    state: GameState,
    field: Grid<Cell, FieldRow, FieldCol>,
}

type TurnData = GridIndex<FieldRow, FieldCol>;

impl FromProtobuf for TurnData {
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
    type TurnData = TurnData;
    type Players = PlayerPool<Player>;

    fn new(players: &[PlayerId]) -> GameResult<Self> {
        let [id1, id2]: [_; 2] = players
            .try_into()
            .map_err(|_| GameError::invalid_players_number(2, players.len()))?;
        if id1 == id2 {
            return Err(GameError::DuplicatePlayerId);
        }
        let p1 = Player::new(id1, Sign::X);
        let p2 = Player::new(id2, Sign::O);
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

        let sign = self.get_current_player()?.sign;
        let cell = self.get_cell(data);
        if cell.is_some() {
            return Err(GameError::cell_is_occupied(
                data.row().into(),
                data.col().into(),
            ));
        }
        *cell = Some(sign);

        self.update_state()
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
    pub fn get_player_by_sign(&self, sign: Sign) -> GameResult<&Player> {
        self.players
            .find(|player| player.sign == sign)
            .ok_or(GameError::PlayerNotFound)
    }

    fn get_cell(&mut self, position: GridIndex<FieldRow, FieldCol>) -> &mut Cell {
        self.field.get_mut_ref(position)
    }

    fn update_state(&mut self) -> GameResult<GameState> {
        let has_sign = |cell: Cell, sign: Sign| matches!(cell, Some(s) if s == sign);
        for i in 0..3 {
            // check rows
            if let [Some(sign1), Some(sign2), Some(sign3)] = self.field[i].as_slice() {
                if sign1 == sign2 && sign2 == sign3 {
                    return self.set_winner_sign(*sign1);
                }
            }
            // check columns
            if let Some(first_sign) = self.field[0][i] {
                if self.field.iter().all(|row| has_sign(row[i], first_sign)) {
                    return self.set_winner_sign(first_sign);
                }
            }
        }
        // check diagonals
        if let (Some(sign1), Some(sign2), Some(sign3)) =
            (self.field[0][0], self.field[1][1], self.field[2][2])
        {
            if sign1 == sign2 && sign2 == sign3 {
                return self.set_winner_sign(sign1);
            }
        }
        if let (Some(sign1), Some(sign2), Some(sign3)) =
            (self.field[0][2], self.field[1][1], self.field[2][0])
        {
            if sign1 == sign2 && sign2 == sign3 {
                return self.set_winner_sign(sign1);
            }
        }

        if self.field.iter().flatten().all(|s| s.is_some()) {
            return Ok(self.set_draw());
        }

        self.switch_player()
    }

    fn set_winner_sign(&mut self, sign: Sign) -> GameResult<GameState> {
        let player = self.get_player_by_sign(sign)?;
        Ok(self.set_winner(player.id))
    }
}
