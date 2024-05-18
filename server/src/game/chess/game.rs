use std::cmp::Ordering;
use std::collections::HashMap;
use std::iter::Scan;

use crate::game::chess::index::{Col, Index, Row};
use crate::game::chess::turn_data::TurnData;
use crate::game::chess::types::{MoveType, Piece, PieceKind, Team};
use crate::game::error::GameError;
use crate::game::game::{Game, GameResult, GameState};
use crate::game::grid::{Grid, WithGridIndex, WithLength};
use crate::game::player_pool::{PlayerId, PlayerPool, PlayerQueue, WithPlayerId};

type Cell = Option<Piece>;

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

fn initial_board(player1: PlayerId, player2: PlayerId) -> Grid<Cell, Row, Col> {
    let mut board = Grid::default();
    // init pawns
    for i in 0..=Col::max().0 {
        *board.get_mut_ref(Index::new(Row::max() - 1, Col(i))) = Some(Piece::create_pawn(player1));
        *board.get_mut_ref(Index::new(Row(1), Col(i))) = Some(Piece::create_pawn(player2));
    }
    // init rooks
    *board.get_mut_ref(Index::new(Row::max(), Col(0))) = Some(Piece::create_rook(player1));
    *board.get_mut_ref(Index::new(Row::max(), Col::max())) = Some(Piece::create_rook(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(0))) = Some(Piece::create_rook(player2));
    *board.get_mut_ref(Index::new(Row(0), Col::max())) = Some(Piece::create_rook(player2));
    // init knights
    *board.get_mut_ref(Index::new(Row::max(), Col(1))) = Some(Piece::create_knight(player1));
    *board.get_mut_ref(Index::new(Row::max(), Col::max() - 1)) =
        Some(Piece::create_knight(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(1))) = Some(Piece::create_knight(player2));
    *board.get_mut_ref(Index::new(Row(0), Col::max() - 1)) = Some(Piece::create_knight(player2));
    // init bishops
    *board.get_mut_ref(Index::new(Row::max(), Col(2))) = Some(Piece::create_bishop(player1));
    *board.get_mut_ref(Index::new(Row::max(), Col::max() - 2)) =
        Some(Piece::create_bishop(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(2))) = Some(Piece::create_bishop(player2));
    *board.get_mut_ref(Index::new(Row(0), Col::max() - 2)) = Some(Piece::create_bishop(player2));
    // init queens
    *board.get_mut_ref(Index::new(Row::max(), Col(3))) = Some(Piece::create_queen(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(3))) = Some(Piece::create_queen(player2));
    // init kings
    *board.get_mut_ref(Index::new(Row::max(), Col(4))) = Some(Piece::create_king(player1));
    *board.get_mut_ref(Index::new(Row(0), Col(4))) = Some(Piece::create_king(player2));

    board
}

// iterator helper
fn until_encounter<'a, I>(
    it: I,
) -> Scan<I, bool, impl FnMut(&mut bool, I::Item) -> Option<I::Item> + 'a>
where
    I: Iterator<Item = (Index, &'a Cell)>,
{
    it.scan(false, |encountered, elem| {
        if *encountered {
            return None;
        }
        *encountered = elem.1.is_some();
        Some(elem)
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Debug, Default)]
struct AdditionalState {
    castle_options: CastleOptions,
    check: Vec<Index>,
    king_pos: Index,
}

impl AdditionalState {
    pub fn new(king_pos: Index) -> Self {
        Self {
            king_pos,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct Chess {
    players: PlayerPool<Player>,
    state: GameState,
    board: Grid<Cell, Row, Col>,
    player_state: HashMap<PlayerId, AdditionalState>,
}

impl Game for Chess {
    type TurnData = TurnData;
    type Players = PlayerPool<Player>;

    fn new(players: &[PlayerId]) -> GameResult<Self> {
        let [id1, id2]: [_; 2] = players
            .try_into()
            .map_err(|_| GameError::invalid_players_number(2, players.len()))?;
        if id1 == id2 {
            return Err(GameError::DuplicatePlayerId);
        }
        let p1 = Player::new(id1, Team::White);
        let p2 = Player::new(id2, Team::Black);
        Ok(Self {
            players: Self::Players::new([p1, p2].to_vec()),
            state: GameState::Turn(p1.id),
            board: initial_board(id1, id2),
            player_state: [
                (
                    p1.id,
                    AdditionalState::new(p1.team.get_king_initial_position()),
                ),
                (
                    p2.id,
                    AdditionalState::new(p2.team.get_king_initial_position()),
                ),
            ]
            .into_iter()
            .collect(),
        })
    }

    fn update(&mut self, id: PlayerId, data: Self::TurnData) -> GameResult<GameState> {
        if self.is_finished() {
            return Err(GameError::GameIsFinished);
        }
        let player = *self.get_current_player()?;
        if id != player.id {
            return Err(GameError::not_your_turn(self.get_current_player()?.id, id));
        }
        let piece = self
            .get_cell_mut(data.from)
            .ok_or(GameError::cell_is_empty(
                data.from.row().into(),
                data.from.col().into(),
            ))?;

        if piece.owner != id {
            return Err(GameError::unauthorized_move(piece.owner, id));
        }
        let available_moves = self.get_moves(data.from)?;
        if !available_moves.contains(&data.to) {
            return Err(GameError::invalid_move(format!(
                "unable to move {} to {}",
                data.from, data.to
            )));
        }

        match self.get_move_type(data) {
            MoveType::LeftCastling => {
                self.move_piece(
                    player.team.get_left_rook_initial_position(),
                    Index::new(data.to.row(), data.to.col() + 1),
                )?;
                self.disable_castling(id);
            }
            MoveType::RightCastling => {
                self.move_piece(
                    player.team.get_right_rook_initial_position(),
                    Index::new(data.to.row(), data.to.col() - 1),
                )?;
                self.disable_castling(id);
            }
            MoveType::KingMove => {
                // castling is disabled inside of update_king_position
                self.update_king_position(id, data.from);
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

impl Chess {
    fn get_cell(&self, position: Index) -> &Cell {
        self.board.get_ref(position)
    }

    fn get_cell_mut(&mut self, position: Index) -> &mut Cell {
        self.board.get_mut_ref(position)
    }

    fn disable_castling(&mut self, id: PlayerId) {
        if let Some(state) = self.player_state.get_mut(&id) {
            state.castle_options = CastleOptions::none();
        }
    }

    fn disable_left_castling(&mut self, id: PlayerId) {
        if let Some(state) = self.player_state.get_mut(&id) {
            state.castle_options.left = false;
        }
    }

    fn disable_right_castling(&mut self, id: PlayerId) {
        if let Some(state) = self.player_state.get_mut(&id) {
            state.castle_options.right = false;
        }
    }

    fn update_king_position(&mut self, id: PlayerId, pos: Index) {
        if let Some(state) = self.player_state.get_mut(&id) {
            state.king_pos = pos;
            // castling is disabled once king has moved
            state.castle_options = CastleOptions::none();
        }
    }

    fn update_check(&mut self, player: &Player) {
        if let Some(king_pos) = self.get_king_position(player.id) {
            let threats = self.get_attack_threats(king_pos, player);
            if let Some(state) = self.player_state.get_mut(&player.id) {
                state.check = threats;
            }
        }
    }

    fn move_piece(&mut self, from: Index, to: Index) -> GameResult<Cell> {
        let piece = self
            .get_cell_mut(from)
            .take()
            .ok_or(GameError::cell_is_empty(
                from.row().into(),
                from.col().into(),
            ))?;
        let old_to = std::mem::replace(self.get_cell_mut(to), Some(piece));
        Ok(old_to)
    }

    fn is_enemy(&self, position: Index, player: PlayerId) -> bool {
        self.get_cell(position)
            .filter(|target| target.is_enemy(player))
            .is_some()
    }

    fn is_friendly(&self, position: Index, player: PlayerId) -> bool {
        self.get_cell(position)
            .filter(|target| !target.is_enemy(player))
            .is_some()
    }

    fn is_in_check(&self, id: PlayerId) -> bool {
        if let Some(threats) = self.player_state.get(&id).map(|state| &state.check) {
            return !threats.is_empty();
        }
        false
    }

    fn get_king_position(&self, id: PlayerId) -> Option<Index> {
        self.player_state.get(&id).map(|state| state.king_pos)
    }

    fn get_move_type(&self, TurnData { from, to }: TurnData) -> MoveType {
        if self.get_cell(from).filter(Piece::is_king).is_some() {
            if (from == Team::Black.get_king_initial_position()
                || from == Team::White.get_king_initial_position())
                && from.row() == to.row()
            {
                match from.col().partial_cmp(&to.col()) {
                    Some(Ordering::Less) if to.col() - 2 == from.col() => {
                        return MoveType::RightCastling;
                    }
                    Some(Ordering::Greater) if from.col() - 2 == to.col() => {
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
        let player_state = self
            .player_state
            .get(&id)
            .ok_or(GameError::PlayerNotFound)?;
        let player = self
            .players
            .find_by_id(id)
            .ok_or(GameError::PlayerNotFound)?;
        let empty_not_threatened = |(pos, cell): (Index, &Cell)| {
            cell.is_none() && self.get_attack_threats(pos, player).is_empty()
        };
        let mut castle_options = player_state.castle_options;
        if player_state.check.is_empty() {
            let king_pos = player.team.get_king_initial_position();
            if castle_options.left {
                let mut left_it = self.board.left_iter(king_pos).indexed().skip(1).take(2);
                castle_options.left = left_it.all(empty_not_threatened);
                if castle_options.left {
                    if let Some(idx) = king_pos.move_left(3) {
                        castle_options.left = self.get_cell(idx).is_none();
                    }
                }
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
                            Team::White => enemy_pos.row() < pos.row(),
                            Team::Black => enemy_pos.row() > pos.row(),
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
        let piece = self
            .get_cell(pos)
            .ok_or(GameError::cell_is_empty(pos.row().into(), pos.col().into()))?;
        let player = *self
            .players
            .find_by_id(piece.owner)
            .ok_or(GameError::PlayerNotFound)?;
        let mut res = vec![];
        let empty_cell_or_enemy = |(index, cell): (Index, &Cell)| {
            if cell.is_none() || matches!(cell, Some(p) if p.is_enemy(piece.owner)) {
                return Some(index);
            }
            None
        };
        match piece.kind {
            PieceKind::Pawn => {
                let advance = match player.team {
                    Team::White => Index::move_up,
                    Team::Black => Index::move_down,
                };
                if let Some(advanced) = advance(&pos, 1) {
                    if self.get_cell(advanced).is_none() {
                        res.push(advanced);
                        // if pawn didn't move it can advance one more row
                        if pos.row() == player.team.get_pawn_initial_row() {
                            if let Some(advanced) = advance(&advanced, 1) {
                                if self.get_cell(advanced).is_none() {
                                    res.push(advanced);
                                }
                            }
                        }
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

        // exclude moves that lead to check
        let king_pos = self
            .get_king_position(player.id)
            .ok_or(GameError::PlayerNotFound)?;
        // TODO: handle errors inside of retain
        res.retain(|&index| {
            let backup = match self.move_piece(pos, index) {
                Ok(cell) => cell,
                Err(_) => return false,
            };
            // if king has moved use it's updated position
            let king_pos = if piece.is_king() { index } else { king_pos };
            let king_safe = self.get_attack_threats(king_pos, &player).is_empty();
            if let Err(_) = self.move_piece(index, pos) {
                return false;
            }
            *self.get_cell_mut(index) = backup;
            king_safe
        });
        Ok(res)
    }

    fn find_pieces_positions(&self, id: PlayerId) -> Vec<Index> {
        let mut pieces = vec![];
        for row in 0..=Row::max().0 {
            for col in 0..=Col::max().0 {
                if let Some(piece) = self.get_cell(Index::new(Row(row), Col(col))) {
                    if !piece.is_enemy(id) {
                        pieces.push(Index::new(Row(row), Col(col)));
                    }
                }
            }
        }
        pieces
    }

    fn update_state(&mut self) -> GameResult<GameState> {
        let current_player = *self.get_current_player()?;
        // player cannot finish its turn in check, so just clear check for current player
        if let Some(state) = self.player_state.get_mut(&current_player.id) {
            state.check.clear();
        }
        let enemy = *self.get_enemy_player()?;
        self.update_check(&enemy);

        let enemy_pieces = self.find_pieces_positions(enemy.id);
        if enemy_pieces.into_iter().all(|index| {
            if let Ok(moves) = self.get_moves(index) {
                return moves.is_empty();
            }
            true
        }) {
            return if self.is_in_check(enemy.id) {
                Ok(self.set_winner(current_player.id))
            } else {
                // stalemate
                Ok(self.set_draw())
            };
        }

        self.switch_player()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use generic_array::typenum::Unsigned;
    use itertools::Itertools;

    use crate::game::game::FinishedState;

    const PLAYER1: u64 = 1;
    const PLAYER2: u64 = 2;

    fn row_indices(row: Row) -> Vec<Index> {
        Grid::<Option<Cell>, Row, Col>::default()
            .right_iter(Index::new(row, Col(0)))
            .indexed()
            .map(|(idx, _)| idx)
            .collect_vec()
    }

    /// returns vector of all possible diagonal moves from a specified position
    fn diagonal_moves(pos: Index) -> Vec<Index> {
        let grid = Grid::<Option<Cell>, Row, Col>::default();
        let top_left = grid.top_left_iter(pos).indexed().skip(1);
        let top_right = grid.top_right_iter(pos).indexed().skip(1);
        let bottom_right = grid.bottom_right_iter(pos).indexed().skip(1);
        let bottom_left = grid.bottom_left_iter(pos).indexed().skip(1);
        top_left
            .chain(top_right)
            .chain(bottom_right)
            .chain(bottom_left)
            .map(|(idx, _)| idx)
            .collect_vec()
    }

    /// returns vector of all possible orthogonal moves from a specified position
    fn orthogonal_moves(pos: Index) -> Vec<Index> {
        let grid = Grid::<Option<Cell>, Row, Col>::default();
        let top = grid.top_iter(pos).indexed().skip(1);
        let right = grid.right_iter(pos).indexed().skip(1);
        let bottom = grid.bottom_iter(pos).indexed().skip(1);
        let left = grid.left_iter(pos).indexed().skip(1);
        top.chain(right)
            .chain(bottom)
            .chain(left)
            .map(|(idx, _)| idx)
            .collect_vec()
    }

    fn sorted<I, T>(it: I) -> impl IntoIterator<Item = T>
    where
        I: IntoIterator<Item = T>,
        T: PartialOrd,
    {
        it.into_iter()
            .sorted_by(|l, r| PartialOrd::partial_cmp(l, r).unwrap())
    }

    fn create_custom_board(players: &[PlayerId], pieces: &[(Index, Piece)]) -> GameResult<Chess> {
        let mut chess = Chess::new(players)?;
        for row in 0..<Row as WithLength>::Length::to_usize() {
            for col in 0..<Col as WithLength>::Length::to_usize() {
                let idx = Index::new(Row(row), Col(col));
                chess.board.get_mut_ref(idx).take();
            }
        }
        for &(idx, piece) in pieces {
            *chess.get_cell_mut(idx) = Some(piece);
        }
        Ok(chess)
    }

    fn create_board_kings_and_rooks_only(
        player1: PlayerId,
        player2: PlayerId,
    ) -> GameResult<Chess> {
        let initial_board: Vec<_> = [(player1, Team::White), (player2, Team::Black)]
            .into_iter()
            .flat_map(|(player, team)| {
                [
                    (team.get_king_initial_position(), Piece::create_king(player)),
                    (
                        team.get_left_rook_initial_position(),
                        Piece::create_rook(player),
                    ),
                    (
                        team.get_right_rook_initial_position(),
                        Piece::create_rook(player),
                    ),
                ]
            })
            .collect();
        create_custom_board(&[player1, player2], &initial_board)
    }

    #[test]
    fn test_creation() {
        // Chess::new with less than players should fail
        assert_eq!(
            Chess::new(&[]).unwrap_err(),
            GameError::invalid_players_number(2, 0)
        );
        assert_eq!(
            Chess::new(&[PLAYER1]).unwrap_err(),
            GameError::invalid_players_number(2, 1)
        );
        // Chess::new with more than players should fail
        assert_eq!(
            Chess::new(&[PLAYER1, PLAYER2, 3]).unwrap_err(),
            GameError::invalid_players_number(2, 3)
        );
        assert_eq!(
            Chess::new(&[PLAYER1, PLAYER2, 3, 9]).unwrap_err(),
            GameError::invalid_players_number(2, 4)
        );
        // Chess::new with duplicated player id should fail
        assert_eq!(
            Chess::new(&[PLAYER1, PLAYER1]).unwrap_err(),
            GameError::DuplicatePlayerId
        );

        let mut chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();
        assert_eq!(chess.get_current_player().unwrap().id, PLAYER1);
        assert_eq!(chess.get_current_player().unwrap().team, Team::White);
        assert_eq!(chess.get_enemy_player().unwrap().id, PLAYER2);
        assert_eq!(chess.get_enemy_player().unwrap().team, Team::Black);
        assert_eq!(chess.state(), GameState::Turn(PLAYER1));

        // check that initial board is correct
        let (p1_backline_expected, p2_backline_expected): (Vec<_>, Vec<_>) = [
            (Piece::create_rook(PLAYER1), Piece::create_rook(PLAYER2)),
            (Piece::create_knight(PLAYER1), Piece::create_knight(PLAYER2)),
            (Piece::create_bishop(PLAYER1), Piece::create_bishop(PLAYER2)),
            (Piece::create_queen(PLAYER1), Piece::create_queen(PLAYER2)),
            (Piece::create_king(PLAYER1), Piece::create_king(PLAYER2)),
            (Piece::create_bishop(PLAYER1), Piece::create_bishop(PLAYER2)),
            (Piece::create_knight(PLAYER1), Piece::create_knight(PLAYER2)),
            (Piece::create_rook(PLAYER1), Piece::create_rook(PLAYER2)),
        ]
        .into_iter()
        .unzip();
        // check that player1 piece set is sound
        let p1_backline_it = chess.board.right_iter(Index::new(Row::max(), Col(0)));
        let p1_pawns_it = chess.board.right_iter(Index::new(Row::max() - 1, Col(0)));
        itertools::assert_equal(
            p1_backline_it.map(|item| item.unwrap()),
            p1_backline_expected.into_iter(),
        );
        itertools::assert_equal(
            p1_pawns_it.map(|item| item.unwrap()),
            std::iter::repeat(Piece::create_pawn(PLAYER1)).take(8),
        );
        // check that player2 piece set is sound
        let p2_backline_it = chess.board.right_iter(Index::new(Row(0), Col(0)));
        let p2_pawns_it = chess.board.right_iter(Index::new(Row(1), Col(0)));
        itertools::assert_equal(
            p2_backline_it.map(|item| item.unwrap()),
            p2_backline_expected.into_iter(),
        );
        itertools::assert_equal(
            p2_pawns_it.map(|item| item.unwrap()),
            std::iter::repeat(Piece::create_pawn(PLAYER2)).take(8),
        );

        // check additional state
        assert_eq!(chess.is_in_check(PLAYER1), false);
        assert_eq!(
            chess.get_king_position(PLAYER1).unwrap(),
            Team::White.get_king_initial_position()
        );
        assert_eq!(
            chess.player_state.get(&PLAYER1).unwrap().castle_options,
            CastleOptions::all()
        );
        assert_eq!(chess.is_in_check(PLAYER2), false);
        assert_eq!(
            chess.get_king_position(PLAYER2).unwrap(),
            Team::Black.get_king_initial_position()
        );
        assert_eq!(
            chess.player_state.get(&PLAYER2).unwrap().castle_options,
            CastleOptions::all()
        );
    }

    #[test]
    fn test_players_switch_turns() {
        let mut chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();

        // check that player1 is the first to make turn
        assert_eq!(chess.get_current_player().unwrap().id, PLAYER1);
        assert_eq!(chess.get_enemy_player().unwrap().id, PLAYER2);

        let h2_index = Index::new(Row::max() - 1, Col::max());
        let turn = TurnData::new(h2_index, h2_index.move_up(1).unwrap());
        chess.update(PLAYER1, turn).unwrap();

        // check that players switched
        assert_eq!(chess.get_current_player().unwrap().id, PLAYER2);
        assert_eq!(chess.get_enemy_player().unwrap().id, PLAYER1);
    }

    #[test]
    fn test_is_enemy_is_friendly() {
        let chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();
        assert!(chess.is_friendly(Team::White.get_king_initial_position(), PLAYER1));
        assert!(chess.is_friendly(Team::Black.get_king_initial_position(), PLAYER2));
        assert!(chess.is_enemy(Team::White.get_king_initial_position(), PLAYER2));
        assert!(chess.is_enemy(Team::Black.get_king_initial_position(), PLAYER1));
    }

    #[test]
    fn test_get_move_type() {
        let [a1, b1, c1, d1, e1, _, g1, _]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let [_, b8, _, _, e8, f8, g8, h8]: [_; 8] = row_indices(Row(0)).try_into().unwrap();
        let f2 = Index::new(Row::max() - 1, Col(5));
        let f3 = Index::new(Row::max() - 2, Col(5));
        let a6 = Index::new(Row(2), Col(0));
        let a7 = Index::new(Row(1), Col(0));
        let mut chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();
        // clear space for black to castle right
        for idx in [f8, g8] {
            chess.get_cell_mut(idx).take();
        }
        // clear space for white to castle left
        for idx in [b1, c1, d1] {
            chess.get_cell_mut(idx).take();
        }

        assert_eq!(
            chess.get_move_type(TurnData::new(e1, c1)),
            MoveType::LeftCastling
        );
        assert_eq!(
            chess.get_move_type(TurnData::new(e8, g8)),
            MoveType::RightCastling
        );
        assert_eq!(
            chess.get_move_type(TurnData::new(e1, d1)),
            MoveType::KingMove
        );
        assert_eq!(
            chess.get_move_type(TurnData::new(e8, f8)),
            MoveType::KingMove
        );
        assert_eq!(
            chess.get_move_type(TurnData::new(a1, d1)),
            MoveType::RookMove
        );
        assert_eq!(
            chess.get_move_type(TurnData::new(h8, f8)),
            MoveType::RookMove
        );
        assert_eq!(chess.get_move_type(TurnData::new(f2, f3)), MoveType::Other);
        assert_eq!(chess.get_move_type(TurnData::new(g1, f3)), MoveType::Other);
        assert_eq!(chess.get_move_type(TurnData::new(a7, a6)), MoveType::Other);
        assert_eq!(chess.get_move_type(TurnData::new(b8, a6)), MoveType::Other);
    }

    #[test]
    fn test_pawn_moves() {
        let a2 = Index::new(Row::max() - 1, Col(0));
        let a3 = a2.move_up(1).unwrap();
        let a4 = a2.move_up(2).unwrap();
        let b7 = Index::new(Row(1), Col(1));
        let b6 = b7.move_down(1).unwrap();
        let b5 = b7.move_down(2).unwrap();
        let b4 = b7.move_down(3).unwrap();
        let b1 = Index::new(Row::max(), Col(1));
        let mut chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();

        // white pawn has two options to move from initial position
        itertools::assert_equal(sorted(chess.get_moves(a2).unwrap()), [a4, a3]);
        // advance pawn by 1
        chess.update(PLAYER1, TurnData::new(a2, a3)).unwrap();

        // black pawn has two options to move from initial position
        itertools::assert_equal(sorted(chess.get_moves(b7).unwrap()), [b6, b5]);
        // advance pawn by 2
        chess.update(PLAYER2, TurnData::new(b7, b5)).unwrap();

        // after pawn has moved it can only advance by one
        itertools::assert_equal(chess.get_moves(a3).unwrap(), [a4]);
        // advance pawn by 1
        chess.update(PLAYER1, TurnData::new(a3, a4)).unwrap();

        // black pawn now can capture white pawn diagonally in addition to moving forward
        itertools::assert_equal(sorted(chess.get_moves(b5).unwrap()), [a4, b4]);
        // capture white pawn
        chess.update(PLAYER2, TurnData::new(b5, a4)).unwrap();

        // black pawn still can advance
        itertools::assert_equal(chess.get_moves(a4).unwrap(), [a3]);
        // create obstacles and check that there is no options for the pawn to move
        chess.update(PLAYER1, TurnData::new(b1, a3)).unwrap();
        assert!(chess.get_moves(a4).unwrap().is_empty());
    }

    /// - pawn protecting from side cannot move
    /// - knight protecting from side cannot move
    /// - rook protecting from side cannot move out of threat line
    /// - bishop protecting diagonally cannot move out of threat line
    /// - queen protecting diagonally cannot move out of threat line
    #[test]
    fn test_protecting_piece_moves_are_limited_by_check() {
        let [a1, b1, c1, d1, ..]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let d2 = Index::new(Row::max() - 1, Col(3));
        let c3 = Index::new(Row::max() - 2, Col(2));
        let b4 = Index::new(Row(4), Col(1));
        let a5 = Index::new(Row(3), Col(0));
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();

        // add threatening rook
        *chess.get_cell_mut(a1) = Some(Piece::create_rook(PLAYER2));

        // add protecting pawn
        *chess.get_cell_mut(c1) = Some(Piece::create_pawn(PLAYER1));
        // white pawn cannot move because it would put king in check
        assert!(chess.get_moves(c1).unwrap().is_empty());

        // add protecting knight
        *chess.get_cell_mut(c1) = Some(Piece::create_knight(PLAYER1));
        // white knight cannot move because it would put king in check
        assert!(chess.get_moves(c1).unwrap().is_empty());

        // add protecting rook
        *chess.get_cell_mut(c1) = Some(Piece::create_rook(PLAYER1));
        // white rook can move only on the threat line
        itertools::assert_equal(sorted(chess.get_moves(c1).unwrap()), [a1, b1, d1]);

        // cleanup
        chess.get_cell_mut(c1).take();
        chess.get_cell_mut(a1).take();
        // new threatening bishop
        *chess.get_cell_mut(a5) = Some(Piece::create_bishop(PLAYER2));

        // add protecting bishop
        *chess.get_cell_mut(c3) = Some(Piece::create_bishop(PLAYER1));
        // white bishop can move only on the threat line
        itertools::assert_equal(sorted(chess.get_moves(c3).unwrap()), [a5, b4, d2]);

        // add protecting queen
        *chess.get_cell_mut(c3) = Some(Piece::create_queen(PLAYER1));
        // white bishop can move only on the threat line
        itertools::assert_equal(sorted(chess.get_moves(c3).unwrap()), [a5, b4, d2]);
    }

    #[test]
    fn test_king_moves_are_limited_by_check() {
        let a8 = Index::new(Row(0), Col(0));
        let [_, _, c1, d1, e1, f1, g1, _]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let [_, _, _, d2, e2, f2, g2, _]: [_; 8] = row_indices(Row::max() - 1).try_into().unwrap();
        let [a3, _, _, _, e3, f3, g3, _]: [_; 8] = row_indices(Row::max() - 2).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();

        // white king has 5 options to move and 2 options for castling
        itertools::assert_equal(
            sorted(chess.get_moves(e1).unwrap()),
            [d2, e2, f2, c1, d1, f1, g1],
        );
        // move diagonally
        chess.update(PLAYER1, TurnData::new(e1, f2)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        // white king has 8 options to move
        itertools::assert_equal(
            sorted(chess.get_moves(f2).unwrap()),
            [e3, f3, g3, e2, g2, e1, f1, g1],
        );
        // move right
        chess.update(PLAYER1, TurnData::new(f2, g2)).unwrap();

        // white king has 5 options to move because of right black rook
        itertools::assert_equal(sorted(chess.get_moves(g2).unwrap()), [f3, g3, f2, f1, g1]);
        // place black rook to cover some of white king's move options
        chess.move_piece(a8, a3).unwrap();
        // skip turn for the second player
        chess.switch_player().unwrap();
        // white king has 3 options to move
        itertools::assert_equal(sorted(chess.get_moves(g2).unwrap()), [f2, f1, g1]);
        // create obstacles and check that there is no options for the king to move
        *chess.get_cell_mut(f2) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(f1) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(g1) = Some(Piece::create_pawn(PLAYER1));
        assert!(chess.get_moves(g2).unwrap().is_empty());
    }

    #[test]
    fn test_king_castling_moves() {
        let [_, _, c1, d1, e1, f1, g1, _]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let [_, _, _, d2, e2, f2, _, _]: [_; 8] = row_indices(Row::max() - 1).try_into().unwrap();
        let [_, _, _, _, e8, f8, g8, _]: [_; 8] = row_indices(Row(0)).try_into().unwrap();
        let [_, _, _, _, e7, f7, _, _]: [_; 8] = row_indices(Row(1)).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
        *chess.get_cell_mut(g1) = Some(Piece::create_knight(PLAYER1));

        // white king has 5 options to move and 1 options for castling
        // because g1 is occupied by knight
        itertools::assert_equal(
            sorted(chess.get_moves(e1).unwrap()),
            [d2, e2, f2, c1, d1, f1],
        );
        // castle left
        chess.update(PLAYER1, TurnData::new(e1, c1)).unwrap();

        // black king has 3 options to move and 1 option for castling
        // because now d8 is checked by the rook
        itertools::assert_equal(sorted(chess.get_moves(e8).unwrap()), [f8, g8, e7, f7]);
        // castle right
        chess.update(PLAYER2, TurnData::new(e8, g8)).unwrap();
    }

    #[test]
    fn test_knight_moves() {
        let [_, b1, c1, _, _, f1, g1, _]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let [_, _, _, d2, e2, _, _, h2]: [_; 8] = row_indices(Row::max() - 1).try_into().unwrap();
        let [_, b3, _, _, _, f3, _, h3]: [_; 8] = row_indices(Row::max() - 2).try_into().unwrap();
        let [_, _, c4, d4, e4, _, _, h4]: [_; 8] = row_indices(Row::max() - 3).try_into().unwrap();
        let [a5, _, c5, _, e5, _, g5, _]: [_; 8] = row_indices(Row::max() - 4).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
        *chess.get_cell_mut(g1) = Some(Piece::create_knight(PLAYER1));

        itertools::assert_equal(sorted(chess.get_moves(g1).unwrap()), [f3, h3, e2]);
        chess.update(PLAYER1, TurnData::new(g1, f3)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(f3).unwrap()),
            [e5, g5, d4, h4, d2, h2, g1],
        );
        chess.update(PLAYER1, TurnData::new(f3, d2)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(d2).unwrap()),
            [c4, e4, b3, f3, b1, f1],
        );
        chess.update(PLAYER1, TurnData::new(d2, b3)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(sorted(chess.get_moves(b3).unwrap()), [a5, c5, d4, d2, c1]);
        // create obstacles and check that there is no options for the knight to move
        *chess.get_cell_mut(a5) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(c5) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(d4) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(d2) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(c1) = Some(Piece::create_pawn(PLAYER1));
        assert!(chess.get_moves(b3).unwrap().is_empty());
    }

    #[test]
    fn test_bishop_moves() {
        let f1 = Index::new(Row::max(), Col(5));
        let a6 = Index::new(Row(2), Col(0));
        let e6 = Index::new(Row(2), Col(4));
        let c8 = Index::new(Row(0), Col(2));
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
        *chess.get_cell_mut(f1) = Some(Piece::create_bishop(PLAYER1));

        itertools::assert_equal(
            sorted(chess.get_moves(f1).unwrap()),
            sorted(diagonal_moves(f1)),
        );
        chess.update(PLAYER1, TurnData::new(f1, a6)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(a6).unwrap()),
            sorted(diagonal_moves(a6)),
        );
        chess.update(PLAYER1, TurnData::new(a6, c8)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(c8).unwrap()),
            sorted(diagonal_moves(c8)),
        );
        chess.update(PLAYER1, TurnData::new(c8, e6)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(e6).unwrap()),
            sorted(diagonal_moves(e6)),
        );
        // create obstacles and check that there is no options for the bishop to move
        *chess.get_cell_mut(e6.move_up(1).and_then(|idx| idx.move_left(1)).unwrap()) =
            Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(e6.move_up(1).and_then(|idx| idx.move_right(1)).unwrap()) =
            Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(e6.move_down(1).and_then(|idx| idx.move_left(1)).unwrap()) =
            Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(e6.move_down(1).and_then(|idx| idx.move_right(1)).unwrap()) =
            Some(Piece::create_pawn(PLAYER1));
        assert!(chess.get_moves(e6).unwrap().is_empty());
    }

    #[test]
    fn test_rook_moves() {
        let a1 = Index::new(Row::max(), Col(0));
        let a4 = Index::new(Row(4), Col(0));
        let d4 = Index::new(Row(4), Col(3));
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(a1).unwrap()),
            sorted(orthogonal_moves(a1))
                .into_iter() // filter out e1, f1, g1, h1
                .filter(|idx| idx.col() < Col(4)),
        );
        chess.update(PLAYER1, TurnData::new(a1, a4)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(a4).unwrap()),
            sorted(orthogonal_moves(a4)),
        );
        chess.update(PLAYER1, TurnData::new(a4, d4)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(d4).unwrap()),
            sorted(orthogonal_moves(d4)),
        );
        // create obstacles and check that there is no options for the rook to move
        *chess.get_cell_mut(d4.move_up(1).unwrap()) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(d4.move_down(1).unwrap()) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(d4.move_right(1).unwrap()) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(d4.move_left(1).unwrap()) = Some(Piece::create_pawn(PLAYER1));
        assert!(chess.get_moves(d4).unwrap().is_empty());
    }

    #[test]
    fn test_queen_moves() {
        let [a1, b1, c1, d1, ..]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let [_, b2, c2, d2, ..]: [_; 8] = row_indices(Row::max() - 1).try_into().unwrap();
        let [_, b3, c3, d3, ..]: [_; 8] = row_indices(Row::max() - 2).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
        *chess.get_cell_mut(d1) = Some(Piece::create_queen(PLAYER1));

        itertools::assert_equal(
            sorted(chess.get_moves(d1).unwrap()),
            sorted(orthogonal_moves(d1).into_iter().chain(diagonal_moves(d1)))
                .into_iter() // filter out a1, e1, f1, g1, h1
                .filter(|&idx| idx != a1 && (idx.col() < Col(4) || idx.row() < Row::max())),
        );
        chess.update(PLAYER1, TurnData::new(d1, c2)).unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(c2).unwrap()),
            sorted(orthogonal_moves(c2).into_iter().chain(diagonal_moves(c2))),
        );
        // create obstacles and check that there is only 3 options left for the queen to move
        *chess.get_cell_mut(b2) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(b3) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(c3) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(d3) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(d2) = Some(Piece::create_pawn(PLAYER1));
        itertools::assert_equal(sorted(chess.get_moves(c2).unwrap()), [b1, c1, d1]);
        // close rest of the options
        *chess.get_cell_mut(b1) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(c1) = Some(Piece::create_pawn(PLAYER1));
        *chess.get_cell_mut(d1) = Some(Piece::create_pawn(PLAYER1));
        assert!(chess.get_moves(c2).unwrap().is_empty());
    }

    #[test]
    fn test_king_move_disables_castling() {
        {
            // king makes a move and it disables ability to castle
            let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions::all()
            );
            let king_pos = Team::White.get_king_initial_position();
            let turn = TurnData::new(king_pos, king_pos.move_up(1).unwrap());
            chess.update(PLAYER1, turn).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions::none()
            );
        }
        {
            // left castling disables castling
            let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions::all()
            );
            let king_pos = Team::White.get_king_initial_position();
            let turn = TurnData::new(king_pos, king_pos.move_left(2).unwrap());
            chess.update(PLAYER1, turn).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions::none()
            );
        }
        {
            // right castling disables castling
            let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions::all()
            );
            let king_pos = Team::White.get_king_initial_position();
            let turn = TurnData::new(king_pos, king_pos.move_right(2).unwrap());
            chess.update(PLAYER1, turn).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions::none()
            );
        }
    }

    #[test]
    fn test_rook_move_disables_castling() {
        {
            // move left rook and check that left castling is disabled afterward
            let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions::all()
            );

            let rook_pos = Team::White.get_left_rook_initial_position();
            let turn = TurnData::new(rook_pos, rook_pos.move_up(1).unwrap());
            chess.update(PLAYER1, turn).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions {
                    left: false,
                    right: true
                }
            );
        }
        {
            // move right rook and check that right castling is disabled afterward
            let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions::all()
            );
            let rook_pos = Team::White.get_right_rook_initial_position();
            let turn = TurnData::new(rook_pos, rook_pos.move_up(1).unwrap());
            chess.update(PLAYER1, turn).unwrap();
            assert_eq!(
                chess.player_state.get(&PLAYER1).unwrap().castle_options,
                CastleOptions {
                    left: true,
                    right: false
                }
            );
        }
    }

    #[test]
    fn test_castling() {
        let [_, b1, c1, d1, e1, f1, g1, _]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let [_, b8, c8, d8, e8, f8, g8, _]: [_; 8] = row_indices(Row(0)).try_into().unwrap();
        {
            // test right castling for both kings
            let mut chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();
            // clear space between kings and right rooks
            for idx in [f1, g1, f8, g8] {
                chess.get_cell_mut(idx).take();
            }

            chess.update(PLAYER1, TurnData::new(e1, g1)).unwrap();
            chess.update(PLAYER2, TurnData::new(e8, g8)).unwrap();

            assert_eq!(*chess.get_cell(g1), Some(Piece::create_king(PLAYER1)));
            assert_eq!(*chess.get_cell(g8), Some(Piece::create_king(PLAYER2)));
            assert_eq!(*chess.get_cell(f1), Some(Piece::create_rook(PLAYER1)));
            assert_eq!(*chess.get_cell(f8), Some(Piece::create_rook(PLAYER2)));
        }
        {
            // test left castling for both kings
            let mut chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();
            // clear space between kings and left rooks
            for idx in [b1, c1, d1, b8, c8, d8] {
                chess.get_cell_mut(idx).take();
            }

            chess.update(PLAYER1, TurnData::new(e1, c1)).unwrap();
            chess.update(PLAYER2, TurnData::new(e8, c8)).unwrap();

            assert_eq!(*chess.get_cell(c1), Some(Piece::create_king(PLAYER1)));
            assert_eq!(*chess.get_cell(c8), Some(Piece::create_king(PLAYER2)));
            assert_eq!(*chess.get_cell(d1), Some(Piece::create_rook(PLAYER1)));
            assert_eq!(*chess.get_cell(d8), Some(Piece::create_rook(PLAYER2)));
        }
    }

    /// castling enabled:
    /// - can_castle returns true for both sides when not passing through check
    /// - can_castle returns true for one side when for the other one king is passing through check
    /// - can_castle returns false for both sides when king is passing through check for each one
    /// - can_castle returns true for one side when
    ///   there's a piece between king and rook on the other side
    /// - can_castle returns false for both sides when there's a piece between king and rook on each one
    /// castling disabled:
    /// - can_castle returns false for both sides when not passing through check and
    ///   there's no piece stands between king and rook on each side
    #[test]
    fn test_can_castle() {
        let [_, b1, _, _, _, _, g1, _]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let [a8, _, c8, _, _, _, g8, h8]: [_; 8] = row_indices(Row(0)).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();

        // castling enabled
        assert_eq!(
            chess.player_state.get(&PLAYER1).unwrap().castle_options,
            CastleOptions::all()
        );
        assert_eq!(chess.can_castle(PLAYER1).unwrap(), CastleOptions::all());

        // black rook at g8 forbids right castling for white king
        chess.move_piece(a8, g8).unwrap();
        assert_eq!(
            chess.can_castle(PLAYER1).unwrap(),
            CastleOptions {
                left: true,
                right: false,
            }
        );

        // black rook at c8 forbids left castling for white king
        chess.move_piece(g8, c8).unwrap();
        assert_eq!(
            chess.can_castle(PLAYER1).unwrap(),
            CastleOptions {
                left: false,
                right: true,
            }
        );

        // black rooks at c8 and g8 forbid castling for both sides for white king
        chess.move_piece(h8, g8).unwrap();
        assert_eq!(chess.can_castle(PLAYER1).unwrap(), CastleOptions::none());

        // cleanup
        chess.get_cell_mut(c8).take();
        chess.get_cell_mut(g8).take();

        // white knight at b1 forbids left castling for white king
        *chess.get_cell_mut(b1) = Some(Piece::create_knight(PLAYER1));
        assert_eq!(
            chess.can_castle(PLAYER1).unwrap(),
            CastleOptions {
                left: false,
                right: true,
            }
        );

        // white knight at g1 forbids right castling for white king
        chess.move_piece(b1, g1).unwrap();
        assert_eq!(
            chess.can_castle(PLAYER1).unwrap(),
            CastleOptions {
                left: true,
                right: false,
            }
        );

        // white knights at b1 and g1 forbid castling for both sides for white king
        *chess.get_cell_mut(b1) = Some(Piece::create_knight(PLAYER1));
        assert_eq!(chess.can_castle(PLAYER1).unwrap(), CastleOptions::none());

        // cleanup
        chess.get_cell_mut(b1).take();
        chess.get_cell_mut(g1).take();

        // castling is still enabled
        assert_eq!(
            chess.player_state.get(&PLAYER1).unwrap().castle_options,
            CastleOptions::all()
        );
        assert_eq!(chess.can_castle(PLAYER1).unwrap(), CastleOptions::all());

        // after castling is disabled can_castle will return false for both sides
        // despite the absence of obstacles
        chess.disable_castling(PLAYER1);
        assert_eq!(chess.can_castle(PLAYER1).unwrap(), CastleOptions::none());
    }

    #[test]
    fn test_check() {
        let d1 = Index::new(Row::max(), Col(3));
        let e1 = Index::new(Row::max(), Col(4));
        let a2 = Index::new(Row::max() - 1, Col(0));
        let f2 = Index::new(Row::max() - 1, Col(5));
        let a8 = Index::new(Row(0), Col(0));
        let d8 = Index::new(Row(0), Col(3));
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
        *chess.get_cell_mut(d8) = Some(Piece::create_queen(PLAYER2));

        // white king is not in check
        chess.update_check(&Player::new(PLAYER1, Team::White));
        assert!(!chess.is_in_check(PLAYER1));
        assert!(chess.player_state.get(&PLAYER1).unwrap().check.is_empty());

        // black queen puts white king in check
        chess.move_piece(d8, d1).unwrap();
        // white king is in check by queen at d1
        chess.update_check(&Player::new(PLAYER1, Team::White));
        assert!(chess.is_in_check(PLAYER1));
        itertools::assert_equal(
            chess.player_state.get(&PLAYER1).unwrap().check.iter(),
            [&d1],
        );

        // white king move out of check
        chess.move_piece(e1, f2).unwrap();
        chess.update_king_position(PLAYER1, f2);
        // white king is not in check
        chess.update_check(&Player::new(PLAYER1, Team::White));
        assert!(!chess.is_in_check(PLAYER1));
        assert!(chess.player_state.get(&PLAYER1).unwrap().check.is_empty());

        // black queen puts white king in check
        chess.move_piece(d1, e1).unwrap();
        // white king is in check by queen at e1
        chess.update_check(&Player::new(PLAYER1, Team::White));
        assert!(chess.is_in_check(PLAYER1));
        itertools::assert_equal(
            chess.player_state.get(&PLAYER1).unwrap().check.iter(),
            [&e1],
        );

        // black rook puts white king in check
        chess.move_piece(a8, a2).unwrap();
        // white king is in check by queen at e1 and rook at a2
        chess.update_check(&Player::new(PLAYER1, Team::White));
        assert!(chess.is_in_check(PLAYER1));
        itertools::assert_equal(
            sorted(chess.player_state.get(&PLAYER1).unwrap().check.iter()),
            [&a2, &e1],
        );
    }

    #[test]
    fn test_get_attack_threats() {
        let [_, _, c1, d1, e1, f1, g1, _]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let [_, _, c2, d2, e2, f2, g2, _]: [_; 8] = row_indices(Row::max() - 1).try_into().unwrap();
        let [_, _, c3, d3, e3, f3, g3, _]: [_; 8] = row_indices(Row::max() - 2).try_into().unwrap();
        let [_, _, _, _, e8, _, _, _]: [_; 8] = row_indices(Row(0)).try_into().unwrap();
        let [_, _, _, d7, e7, f7, _, _]: [_; 8] = row_indices(Row(1)).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();
        let player1 = Player::new(PLAYER1, Team::White);
        let player2 = Player::new(PLAYER2, Team::Black);

        // pawn on the same column is not a threat
        *chess.get_cell_mut(e2) = Some(Piece::create_pawn(PLAYER2));
        assert!(chess.get_attack_threats(e1, &player1).is_empty());
        // pawn on the adjacent column is a threat
        *chess.get_cell_mut(d2) = Some(Piece::create_pawn(PLAYER2));
        itertools::assert_equal(chess.get_attack_threats(e1, &player1), [d2]);
        // add another threatening pawn
        *chess.get_cell_mut(f2) = Some(Piece::create_pawn(PLAYER2));
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [d2, f2]);

        // pawn on the same column is not a threat
        *chess.get_cell_mut(e7) = Some(Piece::create_pawn(PLAYER1));
        assert!(chess.get_attack_threats(e8, &player2).is_empty());
        // pawn on the adjacent column is a threat
        *chess.get_cell_mut(d7) = Some(Piece::create_pawn(PLAYER1));
        itertools::assert_equal(chess.get_attack_threats(e8, &player2), [d7]);
        // add another threatening pawn
        *chess.get_cell_mut(f7) = Some(Piece::create_pawn(PLAYER1));
        itertools::assert_equal(sorted(chess.get_attack_threats(e8, &player2)), [d7, f7]);

        // cleanup
        for idx in [e2, d2, f2, e7, d7, f7] {
            chess.get_cell_mut(idx).take();
        }

        // create knights in front of a white king and check that none of them is a threat
        *chess.get_cell_mut(e2) = Some(Piece::create_knight(PLAYER2));
        *chess.get_cell_mut(d2) = Some(Piece::create_knight(PLAYER2));
        *chess.get_cell_mut(f2) = Some(Piece::create_knight(PLAYER2));
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // knight on c2 is a threat
        *chess.get_cell_mut(c2) = Some(Piece::create_knight(PLAYER2));
        itertools::assert_equal(chess.get_attack_threats(e1, &player1), [c2]);

        // more knights to the god of knights
        *chess.get_cell_mut(d3) = Some(Piece::create_knight(PLAYER2));
        *chess.get_cell_mut(f3) = Some(Piece::create_knight(PLAYER2));
        *chess.get_cell_mut(g2) = Some(Piece::create_knight(PLAYER2));
        itertools::assert_equal(
            sorted(chess.get_attack_threats(e1, &player1)),
            [d3, f3, c2, g2],
        );

        // cleanup
        for idx in [c2, d2, e2, f2, g2, d3, f3] {
            chess.get_cell_mut(idx).take();
        }

        // create black king that is far from the white one and is not a threat
        *chess.get_cell_mut(e3) = Some(Piece::create_king(PLAYER2));
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // create threatening black kings in every possible position
        *chess.get_cell_mut(d1) = Some(Piece::create_king(PLAYER2));
        *chess.get_cell_mut(d2) = Some(Piece::create_king(PLAYER2));
        *chess.get_cell_mut(e2) = Some(Piece::create_king(PLAYER2));
        *chess.get_cell_mut(f2) = Some(Piece::create_king(PLAYER2));
        *chess.get_cell_mut(f1) = Some(Piece::create_king(PLAYER2));
        itertools::assert_equal(
            sorted(chess.get_attack_threats(e1, &player1)),
            [d2, e2, f2, d1, f1],
        );

        // cleanup
        for idx in [d1, f1, d2, e2, f2, d3] {
            chess.get_cell_mut(idx).take();
        }

        // create bishops that are positioned orthogonally and aren't threatening white king
        *chess.get_cell_mut(d1) = Some(Piece::create_bishop(PLAYER2));
        *chess.get_cell_mut(e2) = Some(Piece::create_bishop(PLAYER2));
        *chess.get_cell_mut(f1) = Some(Piece::create_bishop(PLAYER2));
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // create two threatening black bishops
        *chess.get_cell_mut(c3) = Some(Piece::create_bishop(PLAYER2));
        *chess.get_cell_mut(g3) = Some(Piece::create_bishop(PLAYER2));
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [c3, g3]);

        // create one threatening bishop closer to king that one of the other two
        *chess.get_cell_mut(d2) = Some(Piece::create_bishop(PLAYER2));
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [g3, d2]);

        // cleanup
        for idx in [d1, f1, d2, e2, c3, g3] {
            chess.get_cell_mut(idx).take();
        }

        // create rooks that are positioned diagonally and aren't threatening white king
        *chess.get_cell_mut(d2) = Some(Piece::create_rook(PLAYER2));
        *chess.get_cell_mut(f2) = Some(Piece::create_rook(PLAYER2));
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // create three threatening black rooks
        *chess.get_cell_mut(c1) = Some(Piece::create_rook(PLAYER2));
        *chess.get_cell_mut(g1) = Some(Piece::create_rook(PLAYER2));
        *chess.get_cell_mut(e3) = Some(Piece::create_rook(PLAYER2));
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [e3, c1, g1]);

        // create one threatening rook closer to king that one of the other three
        *chess.get_cell_mut(d1) = Some(Piece::create_rook(PLAYER2));
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [e3, d1, g1]);

        // cleanup
        for idx in [c1, d1, g1, d2, f2, e3] {
            chess.get_cell_mut(idx).take();
        }

        // create 4 non-threatening queens
        *chess.get_cell_mut(c2) = Some(Piece::create_queen(PLAYER2));
        *chess.get_cell_mut(g2) = Some(Piece::create_queen(PLAYER2));
        *chess.get_cell_mut(d3) = Some(Piece::create_queen(PLAYER2));
        *chess.get_cell_mut(f3) = Some(Piece::create_queen(PLAYER2));
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // create 5 threatening black queens
        *chess.get_cell_mut(c3) = Some(Piece::create_queen(PLAYER2));
        *chess.get_cell_mut(g3) = Some(Piece::create_queen(PLAYER2));
        *chess.get_cell_mut(c1) = Some(Piece::create_queen(PLAYER2));
        *chess.get_cell_mut(g1) = Some(Piece::create_queen(PLAYER2));
        *chess.get_cell_mut(e3) = Some(Piece::create_queen(PLAYER2));
        itertools::assert_equal(
            sorted(chess.get_attack_threats(e1, &player1)),
            [c3, e3, g3, c1, g1],
        );

        // create two threatening queens closer to king that two of the other five
        *chess.get_cell_mut(f1) = Some(Piece::create_queen(PLAYER2));
        *chess.get_cell_mut(e2) = Some(Piece::create_queen(PLAYER2));
        itertools::assert_equal(
            sorted(chess.get_attack_threats(e1, &player1)),
            [c3, g3, e2, c1, f1],
        );
    }

    #[test]
    fn test_find_pieces_positions() {
        let [a1, b1, c1, d1, e1, f1, g1, h1]: [_; 8] = row_indices(Row::max()).try_into().unwrap();
        let [a2, b2, c2, d2, e2, f2, g2, h2]: [_; 8] =
            row_indices(Row::max() - 1).try_into().unwrap();
        let [a7, b7, c7, d7, e7, f7, g7, h7]: [_; 8] = row_indices(Row(1)).try_into().unwrap();
        let [a8, b8, c8, d8, e8, f8, g8, h8]: [_; 8] = row_indices(Row(0)).try_into().unwrap();
        let mut chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();

        itertools::assert_equal(
            chess.find_pieces_positions(PLAYER1),
            [
                a2, b2, c2, d2, e2, f2, g2, h2, a1, b1, c1, d1, e1, f1, g1, h1,
            ],
        );
        itertools::assert_equal(
            chess.find_pieces_positions(PLAYER2),
            [
                a8, b8, c8, d8, e8, f8, g8, h8, a7, b7, c7, d7, e7, f7, g7, h7,
            ],
        );

        for row in 0..=Row::max().0 {
            for col in 0..=Col::max().0 {
                chess.get_cell_mut(Index::new(Row(row), Col(col))).take();
            }
        }
        itertools::assert_equal(chess.find_pieces_positions(PLAYER1), []);
        itertools::assert_equal(chess.find_pieces_positions(PLAYER2), []);
    }

    #[test]
    fn test_update_state_without_checks_just_switches_turns() {
        let mut chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();

        assert_eq!(chess.state, GameState::Turn(PLAYER1));
        assert!(!chess.is_in_check(PLAYER1));
        assert!(!chess.is_in_check(PLAYER2));
        assert_eq!(chess.update_state().unwrap(), GameState::Turn(PLAYER2));
        assert!(!chess.is_in_check(PLAYER1));
        assert!(!chess.is_in_check(PLAYER2));
        assert_eq!(chess.update_state().unwrap(), GameState::Turn(PLAYER1));
        assert!(!chess.is_in_check(PLAYER1));
        assert!(!chess.is_in_check(PLAYER2));
    }

    #[test]
    fn test_update_state_sets_winner() {
        let a1 = Index::new(Row::max(), Col(0));
        let h1 = Index::new(Row::max(), Col::max());
        let a7 = Index::new(Row(1), Col(0));
        let h8 = Index::new(Row(0), Col::max());
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();

        // rooks on h8 and a7 is checkmate
        chess.move_piece(h1, h8).unwrap();
        chess.move_piece(a1, a7).unwrap();
        assert_eq!(
            chess.update_state().unwrap(),
            GameState::Finished(FinishedState::Win(PLAYER1))
        );
    }

    #[test]
    fn test_update_state_sets_draw() {
        let a1 = Index::new(Row::max(), Col(0));
        let g1 = Index::new(Row::max(), Col::max() - 1);
        let h1 = Index::new(Row::max(), Col::max());
        let a7 = Index::new(Row(1), Col(0));
        let a8 = Index::new(Row(0), Col(0));
        let e8 = Index::new(Row(0), Col(4));
        let h8 = Index::new(Row(0), Col::max());
        let mut chess = create_board_kings_and_rooks_only(PLAYER1, PLAYER2).unwrap();

        // move king to the corner and delete all other black pieces
        chess.get_cell_mut(a8).take();
        chess.move_piece(e8, h8).unwrap();
        chess.update_king_position(PLAYER2, h8);
        // rooks leave black king no option to move, but it's not in check -> stalemate
        chess.move_piece(h1, g1).unwrap();
        chess.move_piece(a1, a7).unwrap();
        assert_eq!(
            chess.update_state().unwrap(),
            GameState::Finished(FinishedState::Draw)
        );
    }

    #[test]
    fn test_update_errors() {
        let [_, _, _, _, e2, _, _, _]: [_; 8] = row_indices(Row::max() - 1).try_into().unwrap();
        let [_, _, _, _, e7, _, _, _]: [_; 8] = row_indices(Row(1)).try_into().unwrap();
        let e3 = Index::new(Row(5), Col(4));
        let e4 = Index::new(Row(4), Col(4));
        let e5 = Index::new(Row(3), Col(4));
        let mut chess = Chess::new(&[PLAYER1, PLAYER2]).unwrap();

        // cannot update finished game
        chess.set_draw();
        assert_eq!(
            chess.update(PLAYER1, TurnData::new(e2, e4)).unwrap_err(),
            GameError::GameIsFinished
        );

        // reset state
        chess.set_state(GameState::Turn(PLAYER1));

        // second player cannot call update while its first player's turn
        assert_eq!(
            chess.update(PLAYER2, TurnData::new(e7, e5)).unwrap_err(),
            GameError::not_your_turn(PLAYER1, PLAYER2)
        );

        // player cannot specify empty cell as 'to'
        assert_eq!(
            chess.update(PLAYER1, TurnData::new(e3, e5)).unwrap_err(),
            GameError::cell_is_empty(e3.row().into(), e3.col().into())
        );

        // player cannot move another player's piece
        assert_eq!(
            chess.update(PLAYER1, TurnData::new(e7, e5)).unwrap_err(),
            GameError::unauthorized_move(PLAYER2, PLAYER1)
        );

        // invalid move
        assert_eq!(
            chess.update(PLAYER1, TurnData::new(e2, e5)).unwrap_err(),
            GameError::invalid_move(format!("unable to move {} to {}", e2, e5))
        );
    }
}
