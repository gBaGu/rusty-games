use generic_array::typenum::Unsigned;
use prost::Message;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::iter::Scan;
use std::ops::{Add, Sub};

use crate::game::error::GameError;
use crate::game::game::{FromProtobuf, FromProtobufError, Game, GameResult};
use crate::game::grid::{Grid, GridIndex, WithGridIndex, WithMaxValue};
use crate::game::player_pool::{PlayerId, PlayerPool, WithPlayerId};
use crate::game::state::{FinishedState, GameState};
use crate::proto::CoordinatesPair;

type Index = GridIndex<Row, Col>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Team {
    Black,
    White,
}

impl Team {
    pub fn get_king_initial_position(&self) -> Index {
        match self {
            Team::White => Index::new(Row(<Row as WithMaxValue>::MaxValue::to_usize() - 1), Col(4)),
            Team::Black => Index::new(Row(0), Col(4)),
        }
    }

    pub fn get_left_rook_initial_position(&self) -> Index {
        let last_row = Row(<Row as WithMaxValue>::MaxValue::to_usize() - 1);
        match self {
            Team::White => Index::new(last_row, Col(0)),
            Team::Black => Index::new(Row(0), Col(0)),
        }
    }

    pub fn get_right_rook_initial_position(&self) -> Index {
        let last_row = Row(<Row as WithMaxValue>::MaxValue::to_usize() - 1);
        let last_col = Col(<Col as WithMaxValue>::MaxValue::to_usize() - 1);
        match self {
            Team::White => Index::new(last_row, last_col),
            Team::Black => Index::new(Row(0), last_col),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Player {
    id: PlayerId,
    team: Team,
}

impl Player {
    pub fn new(id: PlayerId, team: Team) -> Player {
        Self { id, team }
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PieceKind {
    Pawn,
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
    fn create_pawn(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Pawn,
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

    fn is_pawn(&self) -> bool {
        self.kind == PieceKind::Pawn
    }

    fn is_bishop(&self) -> bool {
        self.kind == PieceKind::Bishop
    }

    fn is_knight(&self) -> bool {
        self.kind == PieceKind::Knight
    }

    fn is_rook(&self) -> bool {
        self.kind == PieceKind::Rook
    }

    fn is_queen(&self) -> bool {
        self.kind == PieceKind::Queen
    }

    fn is_king(&self) -> bool {
        self.kind == PieceKind::King
    }

    fn is_enemy(&self, player: PlayerId) -> bool {
        self.owner != player
    }
}

type Cell = Option<Piece>;

enum MoveType {
    LeftCastling,
    RightCastling,
    KingMove,
    RookMove,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub struct TurnData {
    from: Index,
    to: Index,
}

impl TurnData {
    pub fn new(from: Index, to: Index) -> Self {
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
            Index::new(
                Row(usize::try_from(first.row)?),
                Col(usize::try_from(first.col)?),
            ),
            Index::new(
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
        *board.get_mut_ref(Index::new(last_row - 1, Col(i))) = Some(Piece::create_pawn(player1));
        *board.get_mut_ref(Index::new(Row(1), Col(i))) = Some(Piece::create_pawn(player2));
    }
    // init rooks
    *board.get_mut_ref(Index::new(last_row, Col(0))) = Some(Piece::create_rook(player1));
    *board.get_mut_ref(Index::new(last_row, last_col)) = Some(Piece::create_rook(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(0))) = Some(Piece::create_rook(player2));
    *board.get_mut_ref(Index::new(Row(0), last_col)) = Some(Piece::create_rook(player2));
    // init knights
    *board.get_mut_ref(Index::new(last_row, Col(1))) = Some(Piece::create_knight(player1));
    *board.get_mut_ref(Index::new(last_row, last_col - 1)) = Some(Piece::create_knight(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(1))) = Some(Piece::create_knight(player2));
    *board.get_mut_ref(Index::new(Row(0), last_col - 1)) = Some(Piece::create_knight(player2));
    // init bishops
    *board.get_mut_ref(Index::new(last_row, Col(2))) = Some(Piece::create_bishop(player1));
    *board.get_mut_ref(Index::new(last_row, last_col - 2)) = Some(Piece::create_bishop(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(2))) = Some(Piece::create_bishop(player2));
    *board.get_mut_ref(Index::new(Row(0), last_col - 2)) = Some(Piece::create_bishop(player2));
    // init queens
    *board.get_mut_ref(Index::new(last_row, Col(3))) = Some(Piece::create_queen(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(3))) = Some(Piece::create_queen(player2));
    // init kings
    *board.get_mut_ref(Index::new(last_row, Col(4))) = Some(Piece::create_king(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(4))) = Some(Piece::create_king(player2));

    board
}

// iterator helper
fn until_encounter<'a, I: Iterator<Item = (Index, &'a Cell)>>(
    it: I,
) -> Scan<I, bool, impl FnMut(&mut bool, <I as Iterator>::Item) -> Option<(Index, &'a Option<Piece>)>>
{
    it.scan(false, |encountered, elem| {
        if *encountered {
            return None;
        }
        if elem.1.is_some() {
            *encountered = true;
        }
        Some(elem)
    })
}

#[derive(Clone, Copy, Debug)]
struct CastleOptions {
    left: bool,
    right: bool,
}

impl CastleOptions {
    pub fn all() -> Self {
        Self {
            left: true,
            right: true,
        }
    }

    pub fn none() -> Self {
        Self {
            left: false,
            right: false,
        }
    }
}

impl Default for CastleOptions {
    fn default() -> Self {
        Self::all()
    }
}

#[derive(Debug)]
struct AdditionalState {
    castle_options: CastleOptions,
    check: Vec<Index>,
    king_pos: Index,
}

#[derive(Debug)]
pub struct Chess {
    players: PlayerPool<Player>,
    state: GameState,
    board: Grid<Cell, Row, Col>,
    additional_state: HashMap<PlayerId, AdditionalState>,
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
        let p1 = Player::new(id1, Team::White);
        let p2 = Player::new(id2, Team::Black);
        Ok(Self {
            players: PlayerPool::new([p1, p2].to_vec()),
            state: GameState::Turn(p1.id),
            board: initial_board(id1, id2),
            additional_state: [
                (
                    p1.id,
                    AdditionalState {
                        castle_options: CastleOptions::all(),
                        check: vec![],
                        king_pos: p1.team.get_king_initial_position(),
                    },
                ),
                (
                    p2.id,
                    AdditionalState {
                        castle_options: CastleOptions::all(),
                        check: vec![],
                        king_pos: p2.team.get_king_initial_position(),
                    },
                ),
            ]
            .into_iter()
            .collect(),
        })
    }

    fn is_finished(&self) -> bool {
        matches!(self.state, GameState::Finished(_))
    }

    fn update(&mut self, id: PlayerId, data: Self::TurnData) -> GameResult<GameState> {
        if matches!(self.state, GameState::Finished(_)) {
            return Err(GameError::GameIsFinished);
        }
        let player = *self.get_current_player()?;
        if id != player.id {
            return Err(GameError::NotYourTurn {
                expected: self.get_current_player()?.id,
                found: id,
            });
        }
        let piece = self.get_cell_mut(data.from).ok_or(GameError::CellIsEmpty {
            row: data.from.get_row(),
            col: data.from.get_col(),
        })?;

        if piece.owner != id {
            return Err(GameError::UnauthorizedMove {
                expected: piece.owner,
                found: id,
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

        match self.get_move_type(data) {
            MoveType::LeftCastling => {
                self.move_piece(
                    player.team.get_left_rook_initial_position(),
                    Index::new(Row(data.to.get_row()), Col(data.to.get_col() + 1)),
                )?;
                self.disable_castling(id);
            }
            MoveType::RightCastling => {
                self.move_piece(
                    player.team.get_right_rook_initial_position(),
                    Index::new(Row(data.to.get_row()), Col(data.to.get_col() - 1)),
                )?;
                self.disable_castling(id);
            }
            MoveType::KingMove => {
                self.update_king_position(id, data.from);
                self.disable_castling(id)
            }
            MoveType::RookMove => {
                if data.from == player.team.get_left_rook_initial_position() {
                    self.disable_left_castling(id);
                } else if data.from == player.team.get_right_rook_initial_position() {
                    self.disable_right_castling(id);
                }
            }
            MoveType::Other => {}
        };
        self.move_piece(data.from, data.to)?;
        let enemy = *self.get_enemy_player()?;
        self.update_check(&enemy);
        self.update_check(&player);

        self.update_state()
    }
}

impl Chess {
    pub fn get_current_player(&mut self) -> GameResult<&Player> {
        self.players
            .get_current()
            .ok_or(GameError::PlayerPoolCorrupted)
    }
    pub fn get_enemy_player(&mut self) -> GameResult<&Player> {
        let current = *self.get_current_player()?;
        self.players
            .find(|p| p.id != current.id)
            .ok_or(GameError::PlayerPoolCorrupted)
    }

    pub fn get_state(&self) -> &GameState {
        &self.state
    }

    fn get_cell(&self, coordinates: Index) -> &Cell {
        self.board.get_ref(coordinates)
    }

    fn get_cell_mut(&mut self, coordinates: Index) -> &mut Cell {
        self.board.get_mut_ref(coordinates)
    }

    fn disable_castling(&mut self, id: PlayerId) {
        if let Some(state) = self.additional_state.get_mut(&id) {
            state.castle_options = CastleOptions::none();
        }
    }

    fn disable_left_castling(&mut self, id: PlayerId) {
        if let Some(state) = self.additional_state.get_mut(&id) {
            state.castle_options.left = false;
        }
    }

    fn disable_right_castling(&mut self, id: PlayerId) {
        if let Some(state) = self.additional_state.get_mut(&id) {
            state.castle_options.right = false;
        }
    }

    fn update_king_position(&mut self, id: PlayerId, pos: Index) {
        if let Some(state) = self.additional_state.get_mut(&id) {
            state.king_pos = pos;
        }
    }

    fn update_check(&mut self, player: &Player) {
        if let Some(king_pos) = self
            .additional_state
            .get(&player.id)
            .map(|state| state.king_pos)
        {
            let threats = self.get_attack_threats(king_pos, player);
            if let Some(state) = self.additional_state.get_mut(&player.id) {
                state.check = threats;
            }
        }
    }

    fn move_piece(&mut self, from: Index, to: Index) -> GameResult<()> {
        let piece = self
            .get_cell_mut(from)
            .take()
            .ok_or(GameError::CellIsEmpty {
                row: from.get_row(),
                col: from.get_col(),
            })?;
        *self.get_cell_mut(to) = Some(piece);
        Ok(())
    }

    fn is_enemy(&self, coordinates: Index, player: PlayerId) -> bool {
        self.get_cell(coordinates)
            .filter(|target| target.is_enemy(player))
            .is_some()
    }

    fn is_friendly(&self, coordinates: Index, player: PlayerId) -> bool {
        self.get_cell(coordinates)
            .filter(|target| !target.is_enemy(player))
            .is_some()
    }

    fn get_move_type(&self, TurnData { from, to }: TurnData) -> MoveType {
        if self.get_cell(from).filter(Piece::is_king).is_some() {
            if (from == Team::Black.get_king_initial_position()
                || from == Team::White.get_king_initial_position())
                && from.get_row() == to.get_row()
            {
                match from.get_col().partial_cmp(&to.get_col()) {
                    Some(Ordering::Less) if to.get_col() - 2 == from.get_col() => {
                        return MoveType::RightCastling;
                    }
                    Some(Ordering::Greater) if from.get_col() - 2 == to.get_col() => {
                        return MoveType::LeftCastling;
                    }
                    _ => {}
                };
            }
            return MoveType::KingMove;
        }
        if self.get_cell(from).filter(Piece::is_rook).is_some() {
            return MoveType::RookMove;
        }
        MoveType::Other
    }

    fn can_castle(&self, id: PlayerId) -> GameResult<CastleOptions> {
        let additional_state = self
            .additional_state
            .get(&id)
            .ok_or(GameError::PlayerNotFound)?;
        let player = self
            .players
            .find_by_id(id)
            .ok_or(GameError::PlayerNotFound)?;
        let empty_not_threatened = |(pos, cell): (Index, &Cell)| {
            cell.is_none() && self.get_attack_threats(pos, player).is_empty()
        };
        let mut castle_options = additional_state.castle_options;
        if additional_state.check.is_empty() {
            let king_pos = player.team.get_king_initial_position();
            if castle_options.left {
                let mut left_it = self.board.left_iter(king_pos).indexed().skip(1).take(2);
                castle_options.left = left_it.all(empty_not_threatened);
            }
            if castle_options.right {
                let mut right_it = self.board.right_iter(king_pos).indexed().skip(1).take(2);
                castle_options.right = right_it.all(empty_not_threatened);
            }
        }
        Ok(castle_options)
    }

    fn get_attack_threats(&self, pos: Index, player: &Player) -> Vec<Index> {
        let is_occupied = |(pos, cell): (_, &Cell)| {
            if let Some(piece) = cell {
                return Some((pos, *piece));
            }
            None
        };
        let is_enemy = |(_, piece): &(Index, Piece)| piece.is_enemy(player.id);
        let mut diag_tl = self.board.top_left_iter(pos).indexed().skip(1);
        let mut diag_tr = self.board.top_right_iter(pos).indexed().skip(1);
        let mut diag_br = self.board.bottom_right_iter(pos).indexed().skip(1);
        let mut diag_bl = self.board.bottom_left_iter(pos).indexed().skip(1);
        let mut right = self.board.right_iter(pos).indexed().skip(1);
        let mut left = self.board.left_iter(pos).indexed().skip(1);
        let mut top = self.board.top_iter(pos).indexed().skip(1);
        let mut bot = self.board.bottom_iter(pos).indexed().skip(1);

        // get first occupied cell which is enemy (if any) for each diagonal
        let threats = diag_tl
            .find_map(is_occupied)
            .into_iter()
            .filter(is_enemy)
            .chain(diag_tr.find_map(is_occupied).into_iter().filter(is_enemy))
            .chain(diag_br.find_map(is_occupied).into_iter().filter(is_enemy))
            .chain(diag_bl.find_map(is_occupied).into_iter().filter(is_enemy))
            // filter pieces that can attack diagonally
            .filter(|&(enemy_pos, enemy_piece)| match enemy_piece.kind {
                PieceKind::Bishop | PieceKind::Queen => true,
                PieceKind::King => enemy_pos.is_adjacent(&pos),
                PieceKind::Pawn => {
                    enemy_pos.is_adjacent(&pos)
                        && match player.team {
                            Team::White => enemy_pos.get_row() > pos.get_row(),
                            Team::Black => enemy_pos.get_row() < pos.get_row(),
                        }
                }
                _ => false,
            })
            .chain(
                // get first occupied cell which is enemy (if any) for each horizontal and vertical line
                right
                    .find_map(is_occupied)
                    .into_iter()
                    .filter(is_enemy)
                    .chain(left.find_map(is_occupied).into_iter().filter(is_enemy))
                    .chain(top.find_map(is_occupied).into_iter().filter(is_enemy))
                    .chain(bot.find_map(is_occupied).into_iter().filter(is_enemy))
                    // filter pieces that can attack horizontally or vertically
                    .filter(|&(enemy_pos, enemy_piece)| match enemy_piece.kind {
                        PieceKind::Rook | PieceKind::Queen => true,
                        PieceKind::King => enemy_pos.is_adjacent(&pos),
                        _ => false,
                    }),
            )
            .map(|(index, _)| index)
            .chain(
                // check all possible knight positions
                pos.move_up(2)
                    .iter()
                    .chain(pos.move_down(2).iter())
                    .flat_map(|pos| [pos.move_right(1), pos.move_left(1)].into_iter().flatten())
                    .chain(
                        pos.move_right(2)
                            .iter()
                            .chain(pos.move_left(2).iter())
                            .flat_map(|pos| {
                                [pos.move_up(1), pos.move_down(1)].into_iter().flatten()
                            }),
                    )
                    .filter(|&pos| {
                        self.get_cell(pos)
                            .filter(|p| p.is_enemy(player.id) && p.kind == PieceKind::Knight)
                            .is_some()
                    }),
            )
            .collect();

        threats
    }

    fn get_moves(&mut self, pos: Index) -> GameResult<Vec<Index>> {
        let piece = self.get_cell(pos).ok_or(GameError::CellIsEmpty {
            row: pos.get_row(),
            col: pos.get_col(),
        })?;
        let player = *self
            .players
            .find_by_id(piece.owner)
            .ok_or(GameError::PlayerNotFound)?;
        let mut res = vec![];
        let empty_cell_or_enemy = |(index, cell): (Index, &Cell)| {
            if cell.filter(|p| !p.is_enemy(piece.owner)).is_some() {
                return Some(index);
            }
            None
        };
        match piece.kind {
            PieceKind::Pawn => {
                let advanced = match player.team {
                    Team::White => pos.move_up(1),
                    Team::Black => pos.move_down(1),
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
                let diag_tl = until_encounter(self.board.top_left_iter(pos).indexed().skip(1));
                let diag_tr = until_encounter(self.board.top_right_iter(pos).indexed().skip(1));
                let diag_br = until_encounter(self.board.bottom_right_iter(pos).indexed().skip(1));
                let diag_bl = until_encounter(self.board.bottom_left_iter(pos).indexed().skip(1));
                res = diag_tl
                    .chain(diag_tr)
                    .chain(diag_br)
                    .chain(diag_bl)
                    .filter_map(empty_cell_or_enemy)
                    .collect();
            }
            PieceKind::Knight => {
                // add possible vertical moves
                res.extend(
                    pos.move_up(2)
                        .iter()
                        .chain(pos.move_down(2).iter())
                        .flat_map(|pos| [pos.move_right(1), pos.move_left(1)].into_iter().flatten())
                        .filter(|&pos| !self.is_friendly(pos, piece.owner)),
                );
                // add possible horizontal moves
                res.extend(
                    pos.move_right(2)
                        .iter()
                        .chain(pos.move_left(2).iter())
                        .flat_map(|pos| [pos.move_up(1), pos.move_down(1)].into_iter().flatten())
                        .filter(|&pos| !self.is_friendly(pos, piece.owner)),
                );
            }
            PieceKind::Rook => {
                let right = until_encounter(self.board.right_iter(pos).indexed().skip(1));
                let left = until_encounter(self.board.left_iter(pos).indexed().skip(1));
                let top = until_encounter(self.board.top_iter(pos).indexed().skip(1));
                let bot = until_encounter(self.board.bottom_iter(pos).indexed().skip(1));
                res = right
                    .chain(left)
                    .chain(top)
                    .chain(bot)
                    .filter_map(empty_cell_or_enemy)
                    .collect();
            }
            PieceKind::Queen => {
                let diag_tl = until_encounter(self.board.top_left_iter(pos).indexed().skip(1));
                let diag_tr = until_encounter(self.board.top_right_iter(pos).indexed().skip(1));
                let diag_br = until_encounter(self.board.bottom_right_iter(pos).indexed().skip(1));
                let diag_bl = until_encounter(self.board.bottom_left_iter(pos).indexed().skip(1));
                let right = until_encounter(self.board.right_iter(pos).indexed().skip(1));
                let left = until_encounter(self.board.left_iter(pos).indexed().skip(1));
                let top = until_encounter(self.board.top_iter(pos).indexed().skip(1));
                let bot = until_encounter(self.board.bottom_iter(pos).indexed().skip(1));
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
                res = [
                    self.board.top_left_iter(pos).indexed().skip(1).next(),
                    self.board.top_right_iter(pos).indexed().skip(1).next(),
                    self.board.bottom_right_iter(pos).indexed().skip(1).next(),
                    self.board.bottom_left_iter(pos).indexed().skip(1).next(),
                    self.board.right_iter(pos).indexed().skip(1).next(),
                    self.board.left_iter(pos).indexed().skip(1).next(),
                    self.board.top_iter(pos).indexed().skip(1).next(),
                    self.board.bottom_iter(pos).indexed().skip(1).next(),
                ]
                .into_iter()
                .flatten()
                .filter_map(empty_cell_or_enemy)
                .collect();

                let castle_options = self.can_castle(piece.owner)?;
                if castle_options.left {
                    res.extend(pos.move_left(2).into_iter());
                }
                if castle_options.right {
                    res.extend(pos.move_right(2).into_iter());
                }
            }
        };
        let king_pos = self
            .additional_state
            .get(&player.id)
            .ok_or(GameError::PlayerNotFound)?
            .king_pos;
        res.retain(|&index| {
            let backup = self.get_cell(index).clone();
            if let Err(_) = self.move_piece(pos, index) {
                return false;
            }
            let king_safe = self.get_attack_threats(king_pos, &player).is_empty();
            if let Err(_) = self.move_piece(index, pos) {
                return false;
            }
            *self.get_cell_mut(index) = backup;
            king_safe
        });
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
