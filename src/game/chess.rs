use prost::Message;

use crate::game::error::GameError;
use crate::game::game::{FromProtobuf, FromProtobufError, Game, GameResult};
use crate::game::grid::{Grid, GridIndex, WithMaxValue};
use crate::game::player_pool::{PlayerId, PlayerPool, WithPlayerId};
use crate::game::state::{FinishedState, GameState};
use crate::proto::CoordinatesPair;

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
pub struct Row(pub usize);
impl WithMaxValue for Row {
    type MaxValue = generic_array::typenum::U8;
}

impl From<Row> for usize {
    fn from(value: Row) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Col(pub usize);
impl WithMaxValue for Col {
    type MaxValue = generic_array::typenum::U8;
}

impl From<Col> for usize {
    fn from(value: Col) -> Self {
        value.0
    }
}

#[derive(Debug)]
pub enum PieceType {
    Pawn,
    Bishop,
    Knight,
    Rook,
    Queen,
    King,
}

#[derive(Debug)]
pub struct Piece {
    piece: PieceType,
    owner: PlayerId,
}

type Cell = Option<Piece>;

#[derive(Debug)]
pub struct TurnData {
    from: GridIndex<Row, Col>,
    to: GridIndex<Row, Col>,
}

impl TurnData {
    pub fn new(from: GridIndex<Row, Col>, to: GridIndex<Row, Col>) -> Self {
        Self { from, to }
    }
}

impl FromProtobuf for TurnData {
    fn from_protobuf(buf: &[u8]) -> Result<Self, FromProtobufError> {
        let coords = CoordinatesPair::decode(buf)?;
        let first = coords
            .first
            .ok_or_else(|| FromProtobufError::TurnDataMissing {
                missing_field: "first".to_string(),
            })?;
        let second = coords
            .second
            .ok_or_else(|| FromProtobufError::TurnDataMissing {
                missing_field: "second".to_string(),
            })?;
        let turn_data = TurnData::new(
            GridIndex::new(
                Row(usize::try_from(first.row)?),
                Col(usize::try_from(first.col)?),
            ),
            GridIndex::new(
                Row(usize::try_from(second.row)?),
                Col(usize::try_from(second.col)?),
            ),
        );
        Ok(turn_data)
    }
}

#[derive(Debug)]
pub struct Chess {
    players: PlayerPool<Player>,
    state: GameState,
    board: Grid<Cell, Row, Col>,
}

impl Game for Chess {
    type TurnData = TurnData;

    fn new(players: &[PlayerId]) -> GameResult<Self> {
        let [id1, id2]: [_; 2] =
            players
                .try_into()
                .map_err(|_| GameError::InvalidPlayersNumber {
                    expected: 2,
                    found: players.len(),
                })?;
        if id1 == id2 {
            return Err(GameError::DuplicatePlayerId);
        }
        let p1 = Player::new(id1);
        let p2 = Player::new(id2);
        Ok(Self {
            players: PlayerPool::new([p1, p2].to_vec()),
            state: GameState::Turn(p1.id),
            board: Grid::empty(), // TODO: fill the board
        })
    }

    fn is_finished(&self) -> bool {
        matches!(self.state, GameState::Finished(_))
    }

    fn update(&mut self, _player: PlayerId, _data: Self::TurnData) -> GameResult<GameState> {
        todo!()
    }
}

impl Chess {
    pub fn get_current_player(&mut self) -> GameResult<&Player> {
        self.players
            .get_current()
            .ok_or(GameError::PlayerPoolCorrupted)
    }

    pub fn get_state(&self) -> &GameState {
        &self.state
    }

    fn get_cell(&mut self, coordinates: GridIndex<Row, Col>) -> &mut Cell {
        self.board.get_mut_ref(coordinates)
    }

    fn set_winner(&mut self, player: PlayerId) -> GameResult<GameState> {
        self.state = GameState::Finished(FinishedState::Win(player));
        Ok(self.state)
    }

    fn switch_player(&mut self) -> GameResult<GameState> {
        let next_player = self.players.next().ok_or(GameError::PlayerPoolCorrupted)?;
        self.state = GameState::Turn(next_player.id);
        Ok(self.state)
    }

    fn update_state(&mut self) -> GameResult<GameState> {
        todo!()
    }
}
