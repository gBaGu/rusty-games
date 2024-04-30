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
        *board.get_mut_ref(GridIndex::new(last_row - 1, Col(i))) =
            Some(Piece::create_pawn(player1, Direction::Up));
        *board.get_mut_ref(GridIndex::new(Row(1), Col(i))) =
            Some(Piece::create_pawn(player2, Direction::Down));
    }
    // init rooks
    *board.get_mut_ref(GridIndex::new(last_row, Col(0))) = Some(Piece::create_rook(player1));
    *board.get_mut_ref(GridIndex::new(last_row, last_col)) = Some(Piece::create_rook(player1));
    *board.get_mut_ref(GridIndex::new(Row(0), Col(0))) = Some(Piece::create_rook(player2));
    *board.get_mut_ref(GridIndex::new(Row(0), last_col)) = Some(Piece::create_rook(player2));
    // init knights
    *board.get_mut_ref(GridIndex::new(last_row, Col(1))) = Some(Piece::create_knight(player1));
    *board.get_mut_ref(GridIndex::new(last_row, last_col - 1)) =
        Some(Piece::create_knight(player1));
    *board.get_mut_ref(GridIndex::new(Row(0), Col(1))) = Some(Piece::create_knight(player2));
    *board.get_mut_ref(GridIndex::new(Row(0), last_col - 1)) = Some(Piece::create_knight(player2));
    // init bishops
    *board.get_mut_ref(GridIndex::new(last_row, Col(2))) = Some(Piece::create_bishop(player1));
    *board.get_mut_ref(GridIndex::new(last_row, last_col - 2)) =
        Some(Piece::create_bishop(player1));
    *board.get_mut_ref(GridIndex::new(Row(0), Col(2))) = Some(Piece::create_bishop(player2));
    *board.get_mut_ref(GridIndex::new(Row(0), last_col - 2)) = Some(Piece::create_bishop(player2));
    // init queens
    *board.get_mut_ref(GridIndex::new(last_row, Col(3))) = Some(Piece::create_queen(player1));
    *board.get_mut_ref(GridIndex::new(last_row, Col(3))) = Some(Piece::create_queen(player2));
    // init kings
    *board.get_mut_ref(GridIndex::new(last_row, Col(4))) = Some(Piece::create_king(player1));
    *board.get_mut_ref(GridIndex::new(last_row, Col(4))) = Some(Piece::create_king(player2));

    board
}

// iterator helper
fn until_first_encounter<'a>(
    encountered: &mut bool,
    elem: (GridIndex<Row, Col>, &'a Cell),
) -> Option<(GridIndex<Row, Col>, &'a Option<Piece>)> {
    if *encountered {
        return None;
    }
    if elem.1.is_some() {
        *encountered = true;
    }
    Some(elem)
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

    fn update(&mut self, player: PlayerId, data: Self::TurnData) -> GameResult<GameState> {
        if matches!(self.state, GameState::Finished(_)) {
            return Err(GameError::GameIsFinished);
        }
        if player != self.get_current_player()?.id {
            return Err(GameError::NotYourTurn {
                expected: self.get_current_player()?.id,
                found: player,
            });
        }

        if let Some(piece) = self.get_cell_mut(data.from) {
            if piece.owner != player {
                return Err(GameError::UnauthorizedMove {
                    expected: piece.owner,
                    found: player,
                });
            }
            let available_moves = self.get_moves(data.from)?;
            if !available_moves.contains(&data.to) {
                return Err(GameError::InvalidMove {
                    reason: format!(
                        "unable to move to this position ({}, {})",
                        data.to.get_row(),
                        data.to.get_col()
                    ),
                });
            }

            *self.get_cell_mut(data.to) = self.get_cell_mut(data.from).take();
        } else {
            return Err(GameError::CellIsEmpty {
                row: data.from.get_row(),
                col: data.from.get_col(),
            });
        }

        self.update_state()
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

    fn is_friendly(&self, coordinates: GridIndex<Row, Col>, player: PlayerId) -> bool {
        self.get_cell(coordinates)
            .filter(|target| !target.is_enemy(player))
            .is_some()
    }

    fn get_moves(&self, pos: GridIndex<Row, Col>) -> GameResult<Vec<GridIndex<Row, Col>>> {
        let piece = self.get_cell(pos).ok_or(GameError::CellIsEmpty {
            row: pos.get_row(),
            col: pos.get_col(),
        })?;
        let mut res = vec![];
        let empty_cell_or_enemy = |(index, cell): (GridIndex<Row, Col>, &Cell)| {
            if cell.filter(|p| !p.is_enemy(piece.owner)).is_some() {
                return Some(index);
            }
            None
        };
        match piece.kind {
            PieceKind::Pawn(d) => {
                let advanced = match d {
                    Direction::Down => pos.move_down(1),
                    Direction::Up => pos.move_up(1),
                };
                if let Some(advanced) = advanced {
                    if self.get_cell(advanced).is_none() {
                        res.push(advanced);
                    }
                    res.extend(
                        [advanced.move_right(1), advanced.move_left(1)]
                            .into_iter()
                            .flatten()
                            .filter(|&index| self.is_enemy(index, piece.owner)),
                    );
                }
            }
            PieceKind::Bishop => {
                let diag_tl = self
                    .board
                    .top_left_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let diag_tr = self
                    .board
                    .top_right_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let diag_br = self
                    .board
                    .bottom_right_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let diag_bl = self
                    .board
                    .bottom_left_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                res = diag_tl
                    .chain(diag_tr)
                    .chain(diag_br)
                    .chain(diag_bl)
                    .filter_map(empty_cell_or_enemy)
                    .collect();
            }
            PieceKind::Knight => {
                if let Some(up) = pos.move_up(2) {
                    res.extend(
                        [up.move_right(1), up.move_left(1)]
                            .into_iter()
                            .flatten()
                            .filter(|&index| !self.is_friendly(index, piece.owner)),
                    );
                }
                if let Some(down) = pos.move_down(2) {
                    res.extend(
                        [down.move_right(1), down.move_left(1)]
                            .into_iter()
                            .flatten()
                            .filter(|&index| !self.is_friendly(index, piece.owner)),
                    );
                }
                if let Some(right) = pos.move_right(2) {
                    res.extend(
                        [right.move_up(1), right.move_down(1)]
                            .into_iter()
                            .flatten()
                            .filter(|&index| !self.is_friendly(index, piece.owner)),
                    );
                }
                if let Some(left) = pos.move_left(2) {
                    res.extend(
                        [left.move_up(1), left.move_down(1)]
                            .into_iter()
                            .flatten()
                            .filter(|&index| !self.is_friendly(index, piece.owner)),
                    );
                }
            }
            PieceKind::Rook => {
                let right = self
                    .board
                    .right_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let left = self
                    .board
                    .left_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let top = self
                    .board
                    .top_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let bot = self
                    .board
                    .bottom_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                res = right
                    .chain(left)
                    .chain(top)
                    .chain(bot)
                    .filter_map(empty_cell_or_enemy)
                    .collect();
            }
            PieceKind::Queen => {
                let diag_tl = self
                    .board
                    .top_left_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let diag_tr = self
                    .board
                    .top_right_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let diag_br = self
                    .board
                    .bottom_right_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let diag_bl = self
                    .board
                    .bottom_left_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let right = self
                    .board
                    .right_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let left = self
                    .board
                    .left_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let top = self
                    .board
                    .top_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                let bot = self
                    .board
                    .bottom_iter(pos)
                    .indexed()
                    .skip(1)
                    .scan(false, until_first_encounter);
                res = diag_tl
                    .chain(diag_tr)
                    .chain(diag_br)
                    .chain(diag_bl)
                    .chain(right)
                    .chain(left)
                    .chain(top)
                    .chain(bot)
                    .filter_map(empty_cell_or_enemy)
                    .collect();
            }
            PieceKind::King => {
                // TODO: account for castling
                let diag_tl = self.board.top_left_iter(pos).indexed().skip(1).next();
                let diag_tr = self.board.top_right_iter(pos).indexed().skip(1).next();
                let diag_br = self.board.bottom_right_iter(pos).indexed().skip(1).next();
                let diag_bl = self.board.bottom_left_iter(pos).indexed().skip(1).next();
                let right = self.board.right_iter(pos).indexed().skip(1).next();
                let left = self.board.left_iter(pos).indexed().skip(1).next();
                let top = self.board.top_iter(pos).indexed().skip(1).next();
                let bot = self.board.bottom_iter(pos).indexed().skip(1).next();
                res = [diag_tl, diag_tr, diag_br, diag_bl, right, left, top, bot]
                    .iter()
                    .flatten()
                    .cloned()
                    .filter_map(empty_cell_or_enemy)
                    .collect();
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
