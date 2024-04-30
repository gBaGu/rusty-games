use generic_array::typenum::Unsigned;
use prost::Message;
use std::ops::{Add, Sub};

use crate::game::error::GameError;
use crate::game::game::{FromProtobuf, FromProtobufError, Game, GameResult};
use crate::game::grid::{Grid, GridIndex, WithGridIndex, WithMaxValue};
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

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Row(pub usize);
impl WithMaxValue for Row {
    type MaxValue = generic_array::typenum::U8;
}

impl Add<usize> for Row {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.add(rhs))
    }
}

impl Sub<usize> for Row {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0.sub(rhs))
    }
}

impl From<usize> for Row {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<Row> for usize {
    fn from(value: Row) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Col(pub usize);
impl WithMaxValue for Col {
    type MaxValue = generic_array::typenum::U8;
}

impl Add<usize> for Col {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0.add(rhs))
    }
}

impl Sub<usize> for Col {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0.sub(rhs))
    }
}

impl From<usize> for Col {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<Col> for usize {
    fn from(value: Col) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug)]
enum Direction {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug)]
pub enum PieceKind {
    Pawn(Direction),
    Bishop,
    Knight,
    Rook,
    Queen,
    King,
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    kind: PieceKind,
    owner: PlayerId,
}

impl Piece {
    fn create_pawn(owner: PlayerId, direction: Direction) -> Self {
        Self {
            kind: PieceKind::Pawn(direction),
            owner,
        }
    }

    fn create_bishop(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Bishop,
            owner,
        }
    }

    fn create_knight(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Knight,
            owner,
        }
    }

    fn create_rook(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Rook,
            owner,
        }
    }

    fn create_queen(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Queen,
            owner,
        }
    }

    fn create_king(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::King,
            owner,
        }
    }

    fn is_enemy(&self, player: PlayerId) -> bool {
        self.owner != player
    }
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

fn initial_board(player1: PlayerId, player2: PlayerId) -> Grid<Cell, Row, Col> {
    let mut board = Grid::empty();
    let last_row = Row(<Row as WithMaxValue>::MaxValue::to_usize() - 1);
    let last_col = Col(<Col as WithMaxValue>::MaxValue::to_usize() - 1);
    // init pawns
    for i in 0..<Col as WithMaxValue>::MaxValue::to_usize() {
        *board.get_mut_ref(GridIndex::new(last_row - 1, Col(i))) = Some(Piece {
            kind: PieceKind::Pawn(Direction::Up),
            owner: player1,
        });
        *board.get_mut_ref(GridIndex::new(Row(1), Col(i))) = Some(Piece {
            kind: PieceKind::Pawn(Direction::Down),
            owner: player2,
        });
    }
    // init rooks
    *board.get_mut_ref(GridIndex::new(last_row, Col(0))) = Some(Piece {
        kind: PieceKind::Rook,
        owner: player1,
    });
    *board.get_mut_ref(GridIndex::new(last_row, last_col)) = Some(Piece {
        kind: PieceKind::Rook,
        owner: player1,
    });
    *board.get_mut_ref(GridIndex::new(Row(0), Col(0))) = Some(Piece {
        kind: PieceKind::Rook,
        owner: player2,
    });
    *board.get_mut_ref(GridIndex::new(Row(0), last_col)) = Some(Piece {
        kind: PieceKind::Rook,
        owner: player2,
    });
    // init knights
    *board.get_mut_ref(GridIndex::new(last_row, Col(1))) = Some(Piece {
        kind: PieceKind::Knight,
        owner: player1,
    });
    *board.get_mut_ref(GridIndex::new(last_row, last_col - 1)) = Some(Piece {
        kind: PieceKind::Knight,
        owner: player1,
    });
    *board.get_mut_ref(GridIndex::new(Row(0), Col(1))) = Some(Piece {
        kind: PieceKind::Knight,
        owner: player2,
    });
    *board.get_mut_ref(GridIndex::new(Row(0), last_col - 1)) = Some(Piece {
        kind: PieceKind::Knight,
        owner: player2,
    });
    // init bishops
    *board.get_mut_ref(GridIndex::new(last_row, Col(2))) = Some(Piece {
        kind: PieceKind::Bishop,
        owner: player1,
    });
    *board.get_mut_ref(GridIndex::new(last_row, last_col - 2)) = Some(Piece {
        kind: PieceKind::Bishop,
        owner: player1,
    });
    *board.get_mut_ref(GridIndex::new(Row(0), Col(2))) = Some(Piece {
        kind: PieceKind::Bishop,
        owner: player2,
    });
    *board.get_mut_ref(GridIndex::new(Row(0), last_col - 2)) = Some(Piece {
        kind: PieceKind::Bishop,
        owner: player2,
    });
    // init queens
    *board.get_mut_ref(GridIndex::new(last_row, Col(3))) = Some(Piece {
        kind: PieceKind::Queen,
        owner: player1,
    });
    *board.get_mut_ref(GridIndex::new(last_row, Col(3))) = Some(Piece {
        kind: PieceKind::Queen,
        owner: player2,
    });
    // init kings
    *board.get_mut_ref(GridIndex::new(last_row, Col(4))) = Some(Piece {
        kind: PieceKind::King,
        owner: player1,
    });
    *board.get_mut_ref(GridIndex::new(last_row, Col(4))) = Some(Piece {
        kind: PieceKind::King,
        owner: player2,
    });

    board
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
            board: initial_board(id1, id2),
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

    fn get_cell(&self, coordinates: GridIndex<Row, Col>) -> &Cell {
        self.board.get_ref(coordinates)
    }

    fn get_cell_mut(&mut self, coordinates: GridIndex<Row, Col>) -> &mut Cell {
        self.board.get_mut_ref(coordinates)
    }

    fn is_enemy(&self, coordinates: GridIndex<Row, Col>, player: PlayerId) -> bool {
        self.get_cell(coordinates)
            .filter(|target| target.is_enemy(player))
            .is_some()
    }

    fn get_moves(&self, pos: GridIndex<Row, Col>) -> GameResult<Vec<GridIndex<Row, Col>>> {
        let piece = self.get_cell(pos).ok_or(GameError::CellIsEmpty {
            row: pos.get_row(),
            col: pos.get_col(),
        })?;
        let mut res = vec![];
        match piece.kind {
            PieceKind::Pawn(d) => {
                let advanced = match d {
                    Direction::Down => pos.move_down(1),
                    Direction::Up => pos.move_up(1),
                };
                if let Some(advanced) = advanced {
                    res.push(advanced);
                    if let Some(moved) = advanced.move_right(1) {
                        if self.is_enemy(moved, piece.owner) {
                            res.push(moved);
                        }
                    }
                }
                if let Some(advanced) = advanced {
                    res.push(advanced);
                    if let Some(moved) = advanced.move_left(1) {
                        if self.is_enemy(moved, piece.owner) {
                            res.push(moved);
                        }
                    }
                }
            }
            PieceKind::Bishop => {
                let mut diag_tl = self.board.top_left_iter(pos).indexed();
                let mut diag_tr = self.board.top_right_iter(pos).indexed();
                let mut diag_br = self.board.bottom_right_iter(pos).indexed();
                let mut diag_bl = self.board.bottom_left_iter(pos).indexed();
                // skip current position for every iterator
                let _ = diag_tl.next();
                let _ = diag_tr.next();
                let _ = diag_br.next();
                let _ = diag_bl.next();
                let diagonals = diag_tl.chain(diag_tr).chain(diag_br).chain(diag_bl);
                for (index, cell) in diagonals {
                    if let Some(target) = cell {
                        if target.is_enemy(piece.owner) {
                            res.push(index);
                        }
                        break;
                    } else {
                        res.push(index);
                    }
                }
            }
            PieceKind::Knight => {
                todo!()
            }
            PieceKind::Rook => {
                todo!()
            }
            PieceKind::Queen => {
                todo!()
            }
            PieceKind::King => {
                todo!()
            }
        };
        Ok(res)
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
