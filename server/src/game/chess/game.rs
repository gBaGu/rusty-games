use std::cmp::Ordering;
use std::collections::HashMap;

use generic_array::typenum;

use super::iterator::{while_empty, GridExt};
use crate::game::chess::turn_data::TurnData;
use crate::game::chess::types::{MoveType, Piece, PieceKind, Team};
use crate::game::error::GameError;
use crate::game::grid::{Grid, GridIndex};
use crate::game::player_pool::{Player, PlayerDataQueue, PlayerQueue};
use crate::game::{BoardCell, Game, GameResult, GameState, PlayerId};

type Cell = BoardCell<Piece>;

#[derive(Clone, Copy, Debug)]
pub struct PlayerData {
    id: PlayerId,
    team: Team,
}

impl PlayerData {
    pub fn new(id: PlayerId, team: Team) -> PlayerData {
        Self { id, team }
    }
}

impl Player for PlayerData {
    type Id = PlayerId;

    fn id(&self) -> PlayerId {
        self.id
    }
}

fn initial_board(player1: PlayerId, player2: PlayerId) -> Grid<Cell, typenum::U8, typenum::U8> {
    let mut board = Grid::<Cell, _, _>::default();
    // init pawns
    for i in 0..8 {
        *board[GridIndex::new(6, i)] = Piece::create_pawn(player1).into();
        *board[GridIndex::new(1, i)] = Piece::create_pawn(player2).into();
    }
    // init rooks
    *board[GridIndex::new(7, 0)] = Piece::create_rook(player1).into();
    *board[GridIndex::new(7, 7)] = Piece::create_rook(player1).into();
    *board[GridIndex::new(0, 0)] = Piece::create_rook(player2).into();
    *board[GridIndex::new(0, 7)] = Piece::create_rook(player2).into();
    // init knights
    *board[GridIndex::new(7, 1)] = Piece::create_knight(player1).into();
    *board[GridIndex::new(7, 6)] = Piece::create_knight(player1).into();
    *board[GridIndex::new(0, 1)] = Piece::create_knight(player2).into();
    *board[GridIndex::new(0, 6)] = Piece::create_knight(player2).into();
    // init bishops
    *board[GridIndex::new(7, 2)] = Piece::create_bishop(player1).into();
    *board[GridIndex::new(7, 5)] = Piece::create_bishop(player1).into();
    *board[GridIndex::new(0, 2)] = Piece::create_bishop(player2).into();
    *board[GridIndex::new(0, 5)] = Piece::create_bishop(player2).into();
    // init queens
    *board[GridIndex::new(7, 3)] = Piece::create_queen(player1).into();
    *board[GridIndex::new(0, 3)] = Piece::create_queen(player2).into();
    // init kings
    *board[GridIndex::new(7, 4)] = Piece::create_king(player1).into();
    *board[GridIndex::new(0, 4)] = Piece::create_king(player2).into();

    board
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

#[derive(Clone, Debug, Default)]
struct AdditionalState {
    castle_options: CastleOptions,
    check: Vec<GridIndex>,
    king_pos: GridIndex,
}

impl AdditionalState {
    pub fn new(king_pos: GridIndex) -> Self {
        Self {
            king_pos,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug)]
pub struct Chess {
    players: PlayerDataQueue<PlayerData, PlayerId>,
    state: GameState,
    board: Grid<Cell, typenum::U8, typenum::U8>,
    player_state: HashMap<PlayerId, AdditionalState>,
}

impl Game for Chess {
    const NUM_PLAYERS: u8 = 2;
    type TurnData = TurnData;
    type Players = PlayerDataQueue<PlayerData, PlayerId>;
    type Board = Grid<Cell, typenum::U8, typenum::U8>;

    fn new() -> Self {
        let [id1, id2]: [_; 2] = (0..Self::NUM_PLAYERS)
            .map(|id| id.into())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let p1 = PlayerData::new(id1, Team::White);
        let p2 = PlayerData::new(id2, Team::Black);
        Self {
            players: Self::Players::new([p1, p2].to_vec()),
            state: GameState::Turn(0),
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
        }
    }

    fn update(&mut self, id: PlayerId, data: Self::TurnData) -> GameResult<GameState> {
        if self.is_finished() {
            return Err(GameError::GameIsFinished);
        }
        let player = *self.get_current_player()?;
        if id != player.id {
            return Err(GameError::not_your_turn(self.get_current_player()?.id, id));
        }
        let piece = self.board[data.from]
            .ok_or(GameError::cell_is_empty(data.from.row(), data.from.col()))?;

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
                    data.to.move_right(1),
                )?;
                self.disable_castling(id);
            }
            MoveType::RightCastling => {
                self.move_piece(
                    player.team.get_right_rook_initial_position(),
                    data.to.move_left(1),
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

    fn board(&self) -> &Self::Board {
        &self.board
    }

    fn board_mut(&mut self) -> &mut Self::Board {
        &mut self.board
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

    fn set_board(&mut self, board: Self::Board) {
        self.board = board;
    }
}

impl Chess {
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

    fn update_king_position(&mut self, id: PlayerId, pos: GridIndex) {
        if let Some(state) = self.player_state.get_mut(&id) {
            state.king_pos = pos;
            // castling is disabled once king has moved
            state.castle_options = CastleOptions::none();
        }
    }

    fn update_check(&mut self, player: &PlayerData) {
        if let Some(king_pos) = self.get_king_position(player.id) {
            let threats = self.get_attack_threats(king_pos, player);
            if let Some(state) = self.player_state.get_mut(&player.id) {
                state.check = threats;
            }
        }
    }

    fn move_piece(&mut self, from: GridIndex, to: GridIndex) -> GameResult<Cell> {
        let piece = self.board[from]
            .take()
            .ok_or(GameError::cell_is_empty(from.row(), from.col()))?;
        let old_to = std::mem::replace(&mut self.board[to], piece.into());
        Ok(old_to)
    }

    fn is_enemy(&self, position: GridIndex, player: PlayerId) -> bool {
        self.board[position]
            .filter(|target| target.is_enemy(player))
            .is_some()
    }

    fn is_in_check(&self, id: PlayerId) -> bool {
        if let Some(threats) = self.player_state.get(&id).map(|state| &state.check) {
            return !threats.is_empty();
        }
        false
    }

    fn get_king_position(&self, id: PlayerId) -> Option<GridIndex> {
        self.player_state.get(&id).map(|state| state.king_pos)
    }

    fn get_move_type(&self, TurnData { from, to }: TurnData) -> MoveType {
        if self.board[from].filter(Piece::is_king).is_some() {
            if (from == Team::Black.get_king_initial_position()
                || from == Team::White.get_king_initial_position())
                && from.row() == to.row()
            {
                match from.col().partial_cmp(&to.col()) {
                    Some(Ordering::Less) if to.col() == from.col() + 2 => {
                        return MoveType::RightCastling;
                    }
                    Some(Ordering::Greater) if from.col() == to.col() + 2 => {
                        return MoveType::LeftCastling;
                    }
                    _ => {}
                };
            }
            return MoveType::KingMove;
        }
        if self.board[from].filter(Piece::is_rook).is_some() {
            return MoveType::RookMove;
        }
        MoveType::Other
    }

    fn can_castle(&self, id: PlayerId) -> GameResult<CastleOptions> {
        let player_state = self
            .player_state
            .get(&id)
            .ok_or(GameError::PlayerNotFound)?;
        let player = self.players.find(id).ok_or(GameError::PlayerNotFound)?;
        let empty_not_threatened = |(pos, cell): (GridIndex, &Cell)| {
            cell.is_none() && self.get_attack_threats(pos, player).is_empty()
        };
        let mut castle_options = player_state.castle_options;
        if player_state.check.is_empty() {
            let king_pos = player.team.get_king_initial_position();
            if castle_options.left {
                let mut left_it = self.board.left_move_iter(king_pos).take(2);
                castle_options.left = left_it.all(empty_not_threatened);
                if castle_options.left {
                    castle_options.left = self.board[king_pos.move_left(3)].is_none();
                }
            }
            if castle_options.right {
                let mut right_it = self.board.right_move_iter(king_pos).take(2);
                castle_options.right = right_it.all(empty_not_threatened);
            }
        }
        Ok(castle_options)
    }

    fn get_attack_threats(&self, pos: GridIndex, player: &PlayerData) -> Vec<GridIndex> {
        let get_occupied = |(pos, cell): (_, &Cell)| {
            if let BoardCell(Some(piece)) = cell {
                return Some((pos, *piece));
            }
            None
        };
        let occupied_tl = self.board.up_left_move_iter(pos).find_map(get_occupied);
        let occupied_tr = self.board.up_right_move_iter(pos).find_map(get_occupied);
        let occupied_br = self.board.down_right_move_iter(pos).find_map(get_occupied);
        let occupied_bl = self.board.down_left_move_iter(pos).find_map(get_occupied);
        let occupied_right = self.board.right_move_iter(pos).find_map(get_occupied);
        let occupied_left = self.board.left_move_iter(pos).find_map(get_occupied);
        let occupied_up = self.board.up_move_iter(pos).find_map(get_occupied);
        let occupied_down = self.board.down_move_iter(pos).find_map(get_occupied);

        // get first occupied cell which is enemy (if any) for each diagonal
        let threats = occupied_tl
            .into_iter()
            .chain(occupied_tr.into_iter())
            .chain(occupied_br.into_iter())
            .chain(occupied_bl.into_iter())
            // filter pieces that can attack diagonally
            .filter(|&(enemy_pos, piece)| {
                if !piece.is_enemy(player.id) {
                    return false;
                }
                match piece.kind {
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
                }
            })
            .chain(
                // get first occupied cell which is enemy (if any) for each horizontal and vertical line
                occupied_right
                    .into_iter()
                    .chain(occupied_left.into_iter())
                    .chain(occupied_up.into_iter())
                    .chain(occupied_down.into_iter())
                    // filter pieces that can attack horizontally or vertically
                    .filter(|&(enemy_pos, piece)| {
                        if !piece.is_enemy(player.id) {
                            return false;
                        }
                        match piece.kind {
                            PieceKind::Rook | PieceKind::Queen => true,
                            PieceKind::King => enemy_pos.is_adjacent(&pos),
                            _ => false,
                        }
                    }),
            )
            .map(|(index, _)| index)
            .chain(
                // check all possible knight positions
                self.board.knight_move_iter(pos).filter_map(|(pos, _)| {
                    if self.board[pos]
                        .filter(|p| p.is_enemy(player.id) && p.kind == PieceKind::Knight)
                        .is_some()
                    {
                        return Some(pos);
                    }
                    None
                }),
            )
            .collect();

        threats
    }

    fn get_moves(&mut self, pos: GridIndex) -> GameResult<Vec<GridIndex>> {
        let piece = self.board[pos].ok_or(GameError::cell_is_empty(pos.row(), pos.col()))?;
        let player = *self
            .players
            .find(piece.owner)
            .ok_or(GameError::PlayerNotFound)?;
        let mut res = vec![];
        let empty_or_enemy = |(index, cell): (GridIndex, &Cell)| {
            if cell.is_none() || matches!(cell, BoardCell(Some(p)) if p.is_enemy(piece.owner)) {
                return Some(index);
            }
            None
        };
        match piece.kind {
            PieceKind::Pawn => {
                let advance = |pos| match player.team {
                    Team::White => self.board.up_move_iter(pos).next(),
                    Team::Black => self.board.down_move_iter(pos).next(),
                };
                if let Some((idx, cell)) = advance(pos) {
                    if cell.is_none() {
                        res.push(idx);
                        // if pawn didn't move it can advance one more row
                        if pos.row() == player.team.get_pawn_initial_row() {
                            if let Some((idx, cell)) = advance(idx) {
                                if cell.is_none() {
                                    res.push(idx);
                                }
                            }
                        }
                    }
                    let left_it = self.board.right_move_iter(idx).take(1);
                    let right_it = self.board.left_move_iter(idx).take(1);
                    res.extend(left_it.chain(right_it).filter_map(|(index, _)| {
                        if self.is_enemy(index, piece.owner) {
                            return Some(index);
                        }
                        None
                    }));
                }
            }
            PieceKind::Bishop => {
                let diag_tl = while_empty(self.board.up_left_move_iter(pos));
                let diag_tr = while_empty(self.board.up_right_move_iter(pos));
                let diag_br = while_empty(self.board.down_right_move_iter(pos));
                let diag_bl = while_empty(self.board.down_left_move_iter(pos));
                res = diag_tl
                    .chain(diag_tr)
                    .chain(diag_br)
                    .chain(diag_bl)
                    .filter_map(empty_or_enemy)
                    .collect();
            }
            PieceKind::Knight => {
                res.extend(self.board.knight_move_iter(pos).filter_map(empty_or_enemy));
            }
            PieceKind::Rook => {
                let right = while_empty(self.board.right_move_iter(pos));
                let left = while_empty(self.board.left_move_iter(pos));
                let top = while_empty(self.board.up_move_iter(pos));
                let bot = while_empty(self.board.down_move_iter(pos));
                res = right
                    .chain(left)
                    .chain(top)
                    .chain(bot)
                    .filter_map(empty_or_enemy)
                    .collect();
            }
            PieceKind::Queen => {
                let diag_tl = while_empty(self.board.up_left_move_iter(pos));
                let diag_tr = while_empty(self.board.up_right_move_iter(pos));
                let diag_br = while_empty(self.board.down_right_move_iter(pos));
                let diag_bl = while_empty(self.board.down_left_move_iter(pos));
                let right = while_empty(self.board.right_move_iter(pos));
                let left = while_empty(self.board.left_move_iter(pos));
                let top = while_empty(self.board.up_move_iter(pos));
                let bot = while_empty(self.board.down_move_iter(pos));
                res = diag_tl
                    .chain(diag_tr)
                    .chain(diag_br)
                    .chain(diag_bl)
                    .chain(right)
                    .chain(left)
                    .chain(top)
                    .chain(bot)
                    .filter_map(empty_or_enemy)
                    .collect();
            }
            PieceKind::King => {
                res = [
                    self.board.up_left_move_iter(pos).next(),
                    self.board.up_right_move_iter(pos).next(),
                    self.board.down_right_move_iter(pos).next(),
                    self.board.down_left_move_iter(pos).next(),
                    self.board.right_move_iter(pos).next(),
                    self.board.left_move_iter(pos).next(),
                    self.board.up_move_iter(pos).next(),
                    self.board.down_move_iter(pos).next(),
                ]
                .into_iter()
                .flatten()
                .filter_map(empty_or_enemy)
                .collect();

                let castle_options = self.can_castle(piece.owner)?;
                if castle_options.left {
                    res.push(pos.move_left(2));
                }
                if castle_options.right {
                    res.push(pos.move_right(2));
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
            self.board[index] = backup;
            king_safe
        });
        Ok(res)
    }

    fn find_pieces_positions(&self, id: PlayerId) -> Vec<GridIndex> {
        let mut pieces = vec![];
        for row in 0..8 {
            for col in 0..8 {
                if let BoardCell(Some(piece)) = self.board[GridIndex::new(row, col)] {
                    if !piece.is_enemy(id) {
                        pieces.push(GridIndex::new(row, col));
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

    use itertools::Itertools;

    use crate::game::{FinishedState, PlayerId};
    use crate::game::grid::WithGridIndex;

    const FIRST_PLAYER: PlayerId = 0;
    const SECOND_PLAYER: PlayerId = 1;

    fn row_indices(row: usize) -> Vec<GridIndex> {
        Grid::<Option<Cell>, typenum::U8, typenum::U8>::default()
            .right_iter(GridIndex::new(row, 0))
            .indexed()
            .map(|(idx, _)| idx)
            .collect_vec()
    }

    /// returns vector of all possible diagonal moves from a specified position
    fn diagonal_moves(pos: GridIndex) -> Vec<GridIndex> {
        let grid = Grid::<Option<Cell>, typenum::U8, typenum::U8>::default();
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
    fn orthogonal_moves(pos: GridIndex) -> Vec<GridIndex> {
        let grid = Grid::<Option<Cell>, typenum::U8, typenum::U8>::default();
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

    fn create_custom_board(pieces: &[(GridIndex, Piece)]) -> Chess {
        let mut chess = Chess::new();
        for row in 0..8 {
            for col in 0..8 {
                chess.board[GridIndex::new(row, col)].take();
            }
        }
        for &(idx, piece) in pieces {
            chess.board[idx] = piece.into();
        }
        chess
    }

    fn create_board_kings_and_rooks_only() -> Chess {
        let initial_board: Vec<_> = [(FIRST_PLAYER, Team::White), (SECOND_PLAYER, Team::Black)]
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
        create_custom_board(&initial_board)
    }

    #[test]
    fn test_creation() {
        let mut chess = Chess::new();
        assert_eq!(chess.get_current_player().unwrap().id, FIRST_PLAYER);
        assert_eq!(chess.get_current_player().unwrap().team, Team::White);
        assert_eq!(chess.get_enemy_player().unwrap().id, SECOND_PLAYER);
        assert_eq!(chess.get_enemy_player().unwrap().team, Team::Black);
        assert_eq!(chess.state(), GameState::Turn(FIRST_PLAYER));

        // check that initial board is correct
        let (p1_backline_expected, p2_backline_expected): (Vec<_>, Vec<_>) = [
            (
                Piece::create_rook(FIRST_PLAYER),
                Piece::create_rook(SECOND_PLAYER),
            ),
            (
                Piece::create_knight(FIRST_PLAYER),
                Piece::create_knight(SECOND_PLAYER),
            ),
            (
                Piece::create_bishop(FIRST_PLAYER),
                Piece::create_bishop(SECOND_PLAYER),
            ),
            (
                Piece::create_queen(FIRST_PLAYER),
                Piece::create_queen(SECOND_PLAYER),
            ),
            (
                Piece::create_king(FIRST_PLAYER),
                Piece::create_king(SECOND_PLAYER),
            ),
            (
                Piece::create_bishop(FIRST_PLAYER),
                Piece::create_bishop(SECOND_PLAYER),
            ),
            (
                Piece::create_knight(FIRST_PLAYER),
                Piece::create_knight(SECOND_PLAYER),
            ),
            (
                Piece::create_rook(FIRST_PLAYER),
                Piece::create_rook(SECOND_PLAYER),
            ),
        ]
        .into_iter()
        .unzip();
        // check that player1 piece set is sound
        let p1_backline_it = chess.board.right_iter(GridIndex::new(7, 0));
        let p1_pawns_it = chess.board.right_iter(GridIndex::new(6, 0));
        itertools::assert_equal(
            p1_backline_it.map(|item| item.unwrap()),
            p1_backline_expected.into_iter(),
        );
        itertools::assert_equal(
            p1_pawns_it.map(|item| item.unwrap()),
            std::iter::repeat(Piece::create_pawn(FIRST_PLAYER)).take(8),
        );
        // check that player2 piece set is sound
        let p2_backline_it = chess.board.right_iter(GridIndex::new(0, 0));
        let p2_pawns_it = chess.board.right_iter(GridIndex::new(1, 0));
        itertools::assert_equal(
            p2_backline_it.map(|item| item.unwrap()),
            p2_backline_expected.into_iter(),
        );
        itertools::assert_equal(
            p2_pawns_it.map(|item| item.unwrap()),
            std::iter::repeat(Piece::create_pawn(SECOND_PLAYER)).take(8),
        );

        // check additional state
        assert_eq!(chess.is_in_check(FIRST_PLAYER), false);
        assert_eq!(
            chess.get_king_position(FIRST_PLAYER).unwrap(),
            Team::White.get_king_initial_position()
        );
        assert_eq!(
            chess
                .player_state
                .get(&FIRST_PLAYER)
                .unwrap()
                .castle_options,
            CastleOptions::all()
        );
        assert_eq!(chess.is_in_check(SECOND_PLAYER), false);
        assert_eq!(
            chess.get_king_position(SECOND_PLAYER).unwrap(),
            Team::Black.get_king_initial_position()
        );
        assert_eq!(
            chess
                .player_state
                .get(&SECOND_PLAYER)
                .unwrap()
                .castle_options,
            CastleOptions::all()
        );
    }

    #[test]
    fn test_players_switch_turns() {
        let mut chess = Chess::new();

        // check that player1 is the first to make turn
        assert_eq!(chess.get_current_player().unwrap().id, FIRST_PLAYER);
        assert_eq!(chess.get_enemy_player().unwrap().id, SECOND_PLAYER);

        let h2_index = GridIndex::new(6, 7);
        let turn = TurnData::new(h2_index, h2_index.move_up(1));
        chess.update(FIRST_PLAYER, turn).unwrap();

        // check that players switched
        assert_eq!(chess.get_current_player().unwrap().id, SECOND_PLAYER);
        assert_eq!(chess.get_enemy_player().unwrap().id, FIRST_PLAYER);
    }

    #[test]
    fn test_is_enemy() {
        let chess = Chess::new();
        assert!(chess.is_enemy(Team::White.get_king_initial_position(), SECOND_PLAYER));
        assert!(chess.is_enemy(Team::Black.get_king_initial_position(), FIRST_PLAYER));
    }

    #[test]
    fn test_get_move_type() {
        let [a1, b1, c1, d1, e1, _, g1, _]: [_; 8] = row_indices(7).try_into().unwrap();
        let [_, b8, _, _, e8, f8, g8, h8]: [_; 8] = row_indices(0).try_into().unwrap();
        let f2 = GridIndex::new(6, 5);
        let f3 = GridIndex::new(5, 5);
        let a6 = GridIndex::new(2, 0);
        let a7 = GridIndex::new(1, 0);
        let mut chess = Chess::new();
        // clear space for black to castle right
        for idx in [f8, g8] {
            chess.board[idx].take();
        }
        // clear space for white to castle left
        for idx in [b1, c1, d1] {
            chess.board[idx].take();
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
        let a2 = GridIndex::new(6, 0);
        let a3 = a2.move_up(1);
        let a4 = a2.move_up(2);
        let b7 = GridIndex::new(1, 1);
        let b6 = b7.move_down(1);
        let b5 = b7.move_down(2);
        let b4 = b7.move_down(3);
        let b1 = GridIndex::new(7, 1);
        let mut chess = Chess::new();

        // white pawn has two options to move from initial position
        itertools::assert_equal(sorted(chess.get_moves(a2).unwrap()), [a4, a3]);
        // advance pawn by 1
        chess.update(FIRST_PLAYER, TurnData::new(a2, a3)).unwrap();

        // black pawn has two options to move from initial position
        itertools::assert_equal(sorted(chess.get_moves(b7).unwrap()), [b6, b5]);
        // advance pawn by 2
        chess.update(SECOND_PLAYER, TurnData::new(b7, b5)).unwrap();

        // after pawn has moved it can only advance by one
        itertools::assert_equal(chess.get_moves(a3).unwrap(), [a4]);
        // advance pawn by 1
        chess.update(FIRST_PLAYER, TurnData::new(a3, a4)).unwrap();

        // black pawn now can capture white pawn diagonally in addition to moving forward
        itertools::assert_equal(sorted(chess.get_moves(b5).unwrap()), [a4, b4]);
        // capture white pawn
        chess.update(SECOND_PLAYER, TurnData::new(b5, a4)).unwrap();

        // black pawn still can advance
        itertools::assert_equal(chess.get_moves(a4).unwrap(), [a3]);
        // create obstacles and check that there is no options for the pawn to move
        chess.update(FIRST_PLAYER, TurnData::new(b1, a3)).unwrap();
        assert!(chess.get_moves(a4).unwrap().is_empty());
    }

    /// - pawn protecting from side cannot move
    /// - knight protecting from side cannot move
    /// - rook protecting from side cannot move out of threat line
    /// - bishop protecting diagonally cannot move out of threat line
    /// - queen protecting diagonally cannot move out of threat line
    #[test]
    fn test_protecting_piece_moves_are_limited_by_check() {
        let [a1, b1, c1, d1, ..]: [_; 8] = row_indices(7).try_into().unwrap();
        let d2 = GridIndex::new(6, 3);
        let c3 = GridIndex::new(5, 2);
        let b4 = GridIndex::new(4, 1);
        let a5 = GridIndex::new(3, 0);
        let mut chess = create_board_kings_and_rooks_only();

        // add threatening rook
        chess.board[a1] = Piece::create_rook(SECOND_PLAYER).into();

        // add protecting pawn
        chess.board[c1] = Piece::create_pawn(FIRST_PLAYER).into();
        // white pawn cannot move because it would put king in check
        assert!(chess.get_moves(c1).unwrap().is_empty());

        // add protecting knight
        chess.board[c1] = Piece::create_knight(FIRST_PLAYER).into();
        // white knight cannot move because it would put king in check
        assert!(chess.get_moves(c1).unwrap().is_empty());

        // add protecting rook
        chess.board[c1] = Piece::create_rook(FIRST_PLAYER).into();
        // white rook can move only on the threat line
        itertools::assert_equal(sorted(chess.get_moves(c1).unwrap()), [a1, b1, d1]);

        // cleanup
        chess.board[c1].take();
        chess.board[a1].take();
        // new threatening bishop
        chess.board[a5] = Piece::create_bishop(SECOND_PLAYER).into();

        // add protecting bishop
        chess.board[c3] = Piece::create_bishop(FIRST_PLAYER).into();
        // white bishop can move only on the threat line
        itertools::assert_equal(sorted(chess.get_moves(c3).unwrap()), [a5, b4, d2]);

        // add protecting queen
        chess.board[c3] = Piece::create_queen(FIRST_PLAYER).into();
        // white bishop can move only on the threat line
        itertools::assert_equal(sorted(chess.get_moves(c3).unwrap()), [a5, b4, d2]);
    }

    #[test]
    fn test_king_moves_are_limited_by_check() {
        let a8 = GridIndex::new(0, 0);
        let [_, _, c1, d1, e1, f1, g1, _]: [_; 8] = row_indices(7).try_into().unwrap();
        let [_, _, _, d2, e2, f2, g2, _]: [_; 8] = row_indices(6).try_into().unwrap();
        let [a3, _, _, _, e3, f3, g3, _]: [_; 8] = row_indices(5).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only();

        // white king has 5 options to move and 2 options for castling
        itertools::assert_equal(
            sorted(chess.get_moves(e1).unwrap()),
            [d2, e2, f2, c1, d1, f1, g1],
        );
        // move diagonally
        chess.update(FIRST_PLAYER, TurnData::new(e1, f2)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        // white king has 8 options to move
        itertools::assert_equal(
            sorted(chess.get_moves(f2).unwrap()),
            [e3, f3, g3, e2, g2, e1, f1, g1],
        );
        // move right
        chess.update(FIRST_PLAYER, TurnData::new(f2, g2)).unwrap();

        // white king has 5 options to move because of right black rook
        itertools::assert_equal(sorted(chess.get_moves(g2).unwrap()), [f3, g3, f2, f1, g1]);
        // place black rook to cover some of white king's move options
        chess.move_piece(a8, a3).unwrap();
        // skip turn for the second player
        chess.switch_player().unwrap();
        // white king has 3 options to move
        itertools::assert_equal(sorted(chess.get_moves(g2).unwrap()), [f2, f1, g1]);
        // create obstacles and check that there is no options for the king to move
        chess.board[f2] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[f1] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[g1] = Piece::create_pawn(FIRST_PLAYER).into();
        assert!(chess.get_moves(g2).unwrap().is_empty());
    }

    #[test]
    fn test_king_castling_moves() {
        let [_, _, c1, d1, e1, f1, g1, _]: [_; 8] = row_indices(7).try_into().unwrap();
        let [_, _, _, d2, e2, f2, _, _]: [_; 8] = row_indices(6).try_into().unwrap();
        let [_, _, _, _, e8, f8, g8, _]: [_; 8] = row_indices(0).try_into().unwrap();
        let [_, _, _, _, e7, f7, _, _]: [_; 8] = row_indices(1).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only();
        chess.board[g1] = Piece::create_knight(FIRST_PLAYER).into();

        // white king has 5 options to move and 1 options for castling
        // because g1 is occupied by knight
        itertools::assert_equal(
            sorted(chess.get_moves(e1).unwrap()),
            [d2, e2, f2, c1, d1, f1],
        );
        // castle left
        chess.update(FIRST_PLAYER, TurnData::new(e1, c1)).unwrap();

        // black king has 3 options to move and 1 option for castling
        // because now d8 is checked by the rook
        itertools::assert_equal(sorted(chess.get_moves(e8).unwrap()), [f8, g8, e7, f7]);
        // castle right
        chess.update(SECOND_PLAYER, TurnData::new(e8, g8)).unwrap();
    }

    #[test]
    fn test_knight_moves() {
        let [_, b1, c1, _, _, f1, g1, _]: [_; 8] = row_indices(7).try_into().unwrap();
        let [_, _, _, d2, e2, _, _, h2]: [_; 8] = row_indices(6).try_into().unwrap();
        let [_, b3, _, _, _, f3, _, h3]: [_; 8] = row_indices(5).try_into().unwrap();
        let [_, _, c4, d4, e4, _, _, h4]: [_; 8] = row_indices(4).try_into().unwrap();
        let [a5, _, c5, _, e5, _, g5, _]: [_; 8] = row_indices(3).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only();
        chess.board[g1] = Piece::create_knight(FIRST_PLAYER).into();

        itertools::assert_equal(sorted(chess.get_moves(g1).unwrap()), [f3, h3, e2]);
        chess.update(FIRST_PLAYER, TurnData::new(g1, f3)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(f3).unwrap()),
            [e5, g5, d4, h4, d2, h2, g1],
        );
        chess.update(FIRST_PLAYER, TurnData::new(f3, d2)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(d2).unwrap()),
            [c4, e4, b3, f3, b1, f1],
        );
        chess.update(FIRST_PLAYER, TurnData::new(d2, b3)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(sorted(chess.get_moves(b3).unwrap()), [a5, c5, d4, d2, c1]);
        // create obstacles and check that there is no options for the knight to move
        chess.board[a5] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[c5] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[d4] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[d2] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[c1] = Piece::create_pawn(FIRST_PLAYER).into();
        assert!(chess.get_moves(b3).unwrap().is_empty());
    }

    #[test]
    fn test_bishop_moves() {
        let f1 = GridIndex::new(7, 5);
        let a6 = GridIndex::new(2, 0);
        let e6 = GridIndex::new(2, 4);
        let c8 = GridIndex::new(0, 2);
        let mut chess = create_board_kings_and_rooks_only();
        chess.board[f1] = Piece::create_bishop(FIRST_PLAYER).into();

        itertools::assert_equal(
            sorted(chess.get_moves(f1).unwrap()),
            sorted(diagonal_moves(f1)),
        );
        chess.update(FIRST_PLAYER, TurnData::new(f1, a6)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(a6).unwrap()),
            sorted(diagonal_moves(a6)),
        );
        chess.update(FIRST_PLAYER, TurnData::new(a6, c8)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(c8).unwrap()),
            sorted(diagonal_moves(c8)),
        );
        chess.update(FIRST_PLAYER, TurnData::new(c8, e6)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(e6).unwrap()),
            sorted(diagonal_moves(e6)),
        );
        // create obstacles and check that there is no options for the bishop to move
        chess.board[e6.move_up(1).move_left(1)] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[e6.move_up(1).move_right(1)] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[e6.move_down(1).move_left(1)] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[e6.move_down(1).move_right(1)] = Piece::create_pawn(FIRST_PLAYER).into();
        assert!(chess.get_moves(e6).unwrap().is_empty());
    }

    #[test]
    fn test_rook_moves() {
        let a1 = GridIndex::new(7, 0);
        let a4 = GridIndex::new(4, 0);
        let d4 = GridIndex::new(4, 3);
        let mut chess = create_board_kings_and_rooks_only();

        itertools::assert_equal(
            sorted(chess.get_moves(a1).unwrap()),
            sorted(orthogonal_moves(a1))
                .into_iter() // filter out e1, f1, g1, h1
                .filter(|idx| idx.col() < 4),
        );
        chess.update(FIRST_PLAYER, TurnData::new(a1, a4)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(a4).unwrap()),
            sorted(orthogonal_moves(a4)),
        );
        chess.update(FIRST_PLAYER, TurnData::new(a4, d4)).unwrap();

        // skip turn for the second player
        chess.switch_player().unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(d4).unwrap()),
            sorted(orthogonal_moves(d4)),
        );
        // create obstacles and check that there is no options for the rook to move
        chess.board[d4.move_up(1)] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[d4.move_down(1)] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[d4.move_right(1)] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[d4.move_left(1)] = Piece::create_pawn(FIRST_PLAYER).into();
        assert!(chess.get_moves(d4).unwrap().is_empty());
    }

    #[test]
    fn test_queen_moves() {
        let [a1, b1, c1, d1, ..]: [_; 8] = row_indices(7).try_into().unwrap();
        let [_, b2, c2, d2, ..]: [_; 8] = row_indices(6).try_into().unwrap();
        let [_, b3, c3, d3, ..]: [_; 8] = row_indices(5).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only();
        chess.board[d1] = Piece::create_queen(FIRST_PLAYER).into();

        itertools::assert_equal(
            sorted(chess.get_moves(d1).unwrap()),
            sorted(orthogonal_moves(d1).into_iter().chain(diagonal_moves(d1)))
                .into_iter() // filter out a1, e1, f1, g1, h1
                .filter(|&idx| idx != a1 && (idx.col() < 4 || idx.row() < 7)),
        );
        chess.update(FIRST_PLAYER, TurnData::new(d1, c2)).unwrap();

        itertools::assert_equal(
            sorted(chess.get_moves(c2).unwrap()),
            sorted(orthogonal_moves(c2).into_iter().chain(diagonal_moves(c2))),
        );
        // create obstacles and check that there is only 3 options left for the queen to move
        chess.board[b2] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[b3] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[c3] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[d3] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[d2] = Piece::create_pawn(FIRST_PLAYER).into();
        itertools::assert_equal(sorted(chess.get_moves(c2).unwrap()), [b1, c1, d1]);
        // close rest of the options
        chess.board[b1] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[c1] = Piece::create_pawn(FIRST_PLAYER).into();
        chess.board[d1] = Piece::create_pawn(FIRST_PLAYER).into();
        assert!(chess.get_moves(c2).unwrap().is_empty());
    }

    #[test]
    fn test_king_move_disables_castling() {
        {
            // king makes a move and it disables ability to castle
            let mut chess = create_board_kings_and_rooks_only();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions::all()
            );
            let king_pos = Team::White.get_king_initial_position();
            let turn = TurnData::new(king_pos, king_pos.move_up(1));
            chess.update(FIRST_PLAYER, turn).unwrap();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions::none()
            );
        }
        {
            // left castling disables castling
            let mut chess = create_board_kings_and_rooks_only();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions::all()
            );
            let king_pos = Team::White.get_king_initial_position();
            let turn = TurnData::new(king_pos, king_pos.move_left(2));
            chess.update(FIRST_PLAYER, turn).unwrap();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions::none()
            );
        }
        {
            // right castling disables castling
            let mut chess = create_board_kings_and_rooks_only();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions::all()
            );
            let king_pos = Team::White.get_king_initial_position();
            let turn = TurnData::new(king_pos, king_pos.move_right(2));
            chess.update(FIRST_PLAYER, turn).unwrap();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions::none()
            );
        }
    }

    #[test]
    fn test_rook_move_disables_castling() {
        {
            // move left rook and check that left castling is disabled afterward
            let mut chess = create_board_kings_and_rooks_only();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions::all()
            );

            let rook_pos = Team::White.get_left_rook_initial_position();
            let turn = TurnData::new(rook_pos, rook_pos.move_up(1));
            chess.update(FIRST_PLAYER, turn).unwrap();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions {
                    left: false,
                    right: true
                }
            );
        }
        {
            // move right rook and check that right castling is disabled afterward
            let mut chess = create_board_kings_and_rooks_only();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions::all()
            );
            let rook_pos = Team::White.get_right_rook_initial_position();
            let turn = TurnData::new(rook_pos, rook_pos.move_up(1));
            chess.update(FIRST_PLAYER, turn).unwrap();
            assert_eq!(
                chess
                    .player_state
                    .get(&FIRST_PLAYER)
                    .unwrap()
                    .castle_options,
                CastleOptions {
                    left: true,
                    right: false
                }
            );
        }
    }

    #[test]
    fn test_castling() {
        let [_, b1, c1, d1, e1, f1, g1, _]: [_; 8] = row_indices(7).try_into().unwrap();
        let [_, b8, c8, d8, e8, f8, g8, _]: [_; 8] = row_indices(0).try_into().unwrap();
        {
            // test right castling for both kings
            let mut chess = Chess::new();
            // clear space between kings and right rooks
            for idx in [f1, g1, f8, g8] {
                chess.board[idx].take();
            }

            chess.update(FIRST_PLAYER, TurnData::new(e1, g1)).unwrap();
            chess.update(SECOND_PLAYER, TurnData::new(e8, g8)).unwrap();

            assert_eq!(chess.board[g1], Piece::create_king(FIRST_PLAYER).into());
            assert_eq!(chess.board[g8], Piece::create_king(SECOND_PLAYER).into());
            assert_eq!(chess.board[f1], Piece::create_rook(FIRST_PLAYER).into());
            assert_eq!(chess.board[f8], Piece::create_rook(SECOND_PLAYER).into());
        }
        {
            // test left castling for both kings
            let mut chess = Chess::new();
            // clear space between kings and left rooks
            for idx in [b1, c1, d1, b8, c8, d8] {
                chess.board[idx].take();
            }

            chess.update(FIRST_PLAYER, TurnData::new(e1, c1)).unwrap();
            chess.update(SECOND_PLAYER, TurnData::new(e8, c8)).unwrap();

            assert_eq!(chess.board[c1], Piece::create_king(FIRST_PLAYER).into());
            assert_eq!(chess.board[c8], Piece::create_king(SECOND_PLAYER).into());
            assert_eq!(chess.board[d1], Piece::create_rook(FIRST_PLAYER).into());
            assert_eq!(chess.board[d8], Piece::create_rook(SECOND_PLAYER).into());
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
        let [_, b1, _, _, _, _, g1, _]: [_; 8] = row_indices(7).try_into().unwrap();
        let [a8, _, c8, _, _, _, g8, h8]: [_; 8] = row_indices(0).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only();

        // castling enabled
        assert_eq!(
            chess
                .player_state
                .get(&FIRST_PLAYER)
                .unwrap()
                .castle_options,
            CastleOptions::all()
        );
        assert_eq!(
            chess.can_castle(FIRST_PLAYER).unwrap(),
            CastleOptions::all()
        );

        // black rook at g8 forbids right castling for white king
        chess.move_piece(a8, g8).unwrap();
        assert_eq!(
            chess.can_castle(FIRST_PLAYER).unwrap(),
            CastleOptions {
                left: true,
                right: false,
            }
        );

        // black rook at c8 forbids left castling for white king
        chess.move_piece(g8, c8).unwrap();
        assert_eq!(
            chess.can_castle(FIRST_PLAYER).unwrap(),
            CastleOptions {
                left: false,
                right: true,
            }
        );

        // black rooks at c8 and g8 forbid castling for both sides for white king
        chess.move_piece(h8, g8).unwrap();
        assert_eq!(
            chess.can_castle(FIRST_PLAYER).unwrap(),
            CastleOptions::none()
        );

        // cleanup
        chess.board[c8].take();
        chess.board[g8].take();

        // white knight at b1 forbids left castling for white king
        chess.board[b1] = Piece::create_knight(FIRST_PLAYER).into();
        assert_eq!(
            chess.can_castle(FIRST_PLAYER).unwrap(),
            CastleOptions {
                left: false,
                right: true,
            }
        );

        // white knight at g1 forbids right castling for white king
        chess.move_piece(b1, g1).unwrap();
        assert_eq!(
            chess.can_castle(FIRST_PLAYER).unwrap(),
            CastleOptions {
                left: true,
                right: false,
            }
        );

        // white knights at b1 and g1 forbid castling for both sides for white king
        chess.board[b1] = Piece::create_knight(FIRST_PLAYER).into();
        assert_eq!(
            chess.can_castle(FIRST_PLAYER).unwrap(),
            CastleOptions::none()
        );

        // cleanup
        chess.board[b1].take();
        chess.board[g1].take();

        // castling is still enabled
        assert_eq!(
            chess
                .player_state
                .get(&FIRST_PLAYER)
                .unwrap()
                .castle_options,
            CastleOptions::all()
        );
        assert_eq!(
            chess.can_castle(FIRST_PLAYER).unwrap(),
            CastleOptions::all()
        );

        // after castling is disabled can_castle will return false for both sides
        // despite the absence of obstacles
        chess.disable_castling(FIRST_PLAYER);
        assert_eq!(
            chess.can_castle(FIRST_PLAYER).unwrap(),
            CastleOptions::none()
        );
    }

    #[test]
    fn test_check() {
        let d1 = GridIndex::new(7, 3);
        let e1 = GridIndex::new(7, 4);
        let a2 = GridIndex::new(6, 0);
        let f2 = GridIndex::new(6, 5);
        let a8 = GridIndex::new(0, 0);
        let d8 = GridIndex::new(0, 3);
        let mut chess = create_board_kings_and_rooks_only();
        chess.board[d8] = Piece::create_queen(SECOND_PLAYER).into();

        // white king is not in check
        chess.update_check(&PlayerData::new(FIRST_PLAYER, Team::White));
        assert!(!chess.is_in_check(FIRST_PLAYER));
        assert!(chess
            .player_state
            .get(&FIRST_PLAYER)
            .unwrap()
            .check
            .is_empty());

        // black queen puts white king in check
        chess.move_piece(d8, d1).unwrap();
        // white king is in check by queen at d1
        chess.update_check(&PlayerData::new(FIRST_PLAYER, Team::White));
        assert!(chess.is_in_check(FIRST_PLAYER));
        itertools::assert_equal(
            chess.player_state.get(&FIRST_PLAYER).unwrap().check.iter(),
            [&d1],
        );

        // white king move out of check
        chess.move_piece(e1, f2).unwrap();
        chess.update_king_position(FIRST_PLAYER, f2);
        // white king is not in check
        chess.update_check(&PlayerData::new(FIRST_PLAYER, Team::White));
        assert!(!chess.is_in_check(FIRST_PLAYER));
        assert!(chess
            .player_state
            .get(&FIRST_PLAYER)
            .unwrap()
            .check
            .is_empty());

        // black queen puts white king in check
        chess.move_piece(d1, e1).unwrap();
        // white king is in check by queen at e1
        chess.update_check(&PlayerData::new(FIRST_PLAYER, Team::White));
        assert!(chess.is_in_check(FIRST_PLAYER));
        itertools::assert_equal(
            chess.player_state.get(&FIRST_PLAYER).unwrap().check.iter(),
            [&e1],
        );

        // black rook puts white king in check
        chess.move_piece(a8, a2).unwrap();
        // white king is in check by queen at e1 and rook at a2
        chess.update_check(&PlayerData::new(FIRST_PLAYER, Team::White));
        assert!(chess.is_in_check(FIRST_PLAYER));
        itertools::assert_equal(
            sorted(chess.player_state.get(&FIRST_PLAYER).unwrap().check.iter()),
            [&a2, &e1],
        );
    }

    #[test]
    fn test_get_attack_threats() {
        let [_, _, c1, d1, e1, f1, g1, _]: [_; 8] = row_indices(7).try_into().unwrap();
        let [_, _, c2, d2, e2, f2, g2, _]: [_; 8] = row_indices(6).try_into().unwrap();
        let [_, _, c3, d3, e3, f3, g3, _]: [_; 8] = row_indices(5).try_into().unwrap();
        let [_, _, _, _, e8, _, _, _]: [_; 8] = row_indices(0).try_into().unwrap();
        let [_, _, _, d7, e7, f7, _, _]: [_; 8] = row_indices(1).try_into().unwrap();
        let mut chess = create_board_kings_and_rooks_only();
        let player1 = PlayerData::new(FIRST_PLAYER, Team::White);
        let player2 = PlayerData::new(SECOND_PLAYER, Team::Black);

        // pawn on the same column is not a threat
        chess.board[e2] = Piece::create_pawn(SECOND_PLAYER).into();
        assert!(chess.get_attack_threats(e1, &player1).is_empty());
        // pawn on the adjacent column is a threat
        chess.board[d2] = Piece::create_pawn(SECOND_PLAYER).into();
        itertools::assert_equal(chess.get_attack_threats(e1, &player1), [d2]);
        // add another threatening pawn
        chess.board[f2] = Piece::create_pawn(SECOND_PLAYER).into();
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [d2, f2]);

        // pawn on the same column is not a threat
        chess.board[e7] = Piece::create_pawn(FIRST_PLAYER).into();
        assert!(chess.get_attack_threats(e8, &player2).is_empty());
        // pawn on the adjacent column is a threat
        chess.board[d7] = Piece::create_pawn(FIRST_PLAYER).into();
        itertools::assert_equal(chess.get_attack_threats(e8, &player2), [d7]);
        // add another threatening pawn
        chess.board[f7] = Piece::create_pawn(FIRST_PLAYER).into();
        itertools::assert_equal(sorted(chess.get_attack_threats(e8, &player2)), [d7, f7]);

        // cleanup
        for idx in [e2, d2, f2, e7, d7, f7] {
            chess.board[idx].take();
        }

        // create knights in front of a white king and check that none of them is a threat
        chess.board[e2] = Piece::create_knight(SECOND_PLAYER).into();
        chess.board[d2] = Piece::create_knight(SECOND_PLAYER).into();
        chess.board[f2] = Piece::create_knight(SECOND_PLAYER).into();
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // knight on c2 is a threat
        chess.board[c2] = Piece::create_knight(SECOND_PLAYER).into();
        itertools::assert_equal(chess.get_attack_threats(e1, &player1), [c2]);

        // more knights to the god of knights
        chess.board[d3] = Piece::create_knight(SECOND_PLAYER).into();
        chess.board[f3] = Piece::create_knight(SECOND_PLAYER).into();
        chess.board[g2] = Piece::create_knight(SECOND_PLAYER).into();
        itertools::assert_equal(
            sorted(chess.get_attack_threats(e1, &player1)),
            [d3, f3, c2, g2],
        );

        // cleanup
        for idx in [c2, d2, e2, f2, g2, d3, f3] {
            chess.board[idx].take();
        }

        // create black king that is far from the white one and is not a threat
        chess.board[e3] = Piece::create_king(SECOND_PLAYER).into();
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // create threatening black kings in every possible position
        chess.board[d1] = Piece::create_king(SECOND_PLAYER).into();
        chess.board[d2] = Piece::create_king(SECOND_PLAYER).into();
        chess.board[e2] = Piece::create_king(SECOND_PLAYER).into();
        chess.board[f2] = Piece::create_king(SECOND_PLAYER).into();
        chess.board[f1] = Piece::create_king(SECOND_PLAYER).into();
        itertools::assert_equal(
            sorted(chess.get_attack_threats(e1, &player1)),
            [d2, e2, f2, d1, f1],
        );

        // cleanup
        for idx in [d1, f1, d2, e2, f2, d3] {
            chess.board[idx].take();
        }

        // create bishops that are positioned orthogonally and aren't threatening white king
        chess.board[d1] = Piece::create_bishop(SECOND_PLAYER).into();
        chess.board[e2] = Piece::create_bishop(SECOND_PLAYER).into();
        chess.board[f1] = Piece::create_bishop(SECOND_PLAYER).into();
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // create two threatening black bishops
        chess.board[c3] = Piece::create_bishop(SECOND_PLAYER).into();
        chess.board[g3] = Piece::create_bishop(SECOND_PLAYER).into();
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [c3, g3]);

        // create one threatening bishop closer to king that one of the other two
        chess.board[d2] = Piece::create_bishop(SECOND_PLAYER).into();
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [g3, d2]);

        // cleanup
        for idx in [d1, f1, d2, e2, c3, g3] {
            chess.board[idx].take();
        }

        // create rooks that are positioned diagonally and aren't threatening white king
        chess.board[d2] = Piece::create_rook(SECOND_PLAYER).into();
        chess.board[f2] = Piece::create_rook(SECOND_PLAYER).into();
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // create three threatening black rooks
        chess.board[c1] = Piece::create_rook(SECOND_PLAYER).into();
        chess.board[g1] = Piece::create_rook(SECOND_PLAYER).into();
        chess.board[e3] = Piece::create_rook(SECOND_PLAYER).into();
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [e3, c1, g1]);

        // create one threatening rook closer to king that one of the other three
        chess.board[d1] = Piece::create_rook(SECOND_PLAYER).into();
        itertools::assert_equal(sorted(chess.get_attack_threats(e1, &player1)), [e3, d1, g1]);

        // cleanup
        for idx in [c1, d1, g1, d2, f2, e3] {
            chess.board[idx].take();
        }

        // create 4 non-threatening queens
        chess.board[c2] = Piece::create_queen(SECOND_PLAYER).into();
        chess.board[g2] = Piece::create_queen(SECOND_PLAYER).into();
        chess.board[d3] = Piece::create_queen(SECOND_PLAYER).into();
        chess.board[f3] = Piece::create_queen(SECOND_PLAYER).into();
        assert!(chess.get_attack_threats(e1, &player1).is_empty());

        // create 5 threatening black queens
        chess.board[c3] = Piece::create_queen(SECOND_PLAYER).into();
        chess.board[g3] = Piece::create_queen(SECOND_PLAYER).into();
        chess.board[c1] = Piece::create_queen(SECOND_PLAYER).into();
        chess.board[g1] = Piece::create_queen(SECOND_PLAYER).into();
        chess.board[e3] = Piece::create_queen(SECOND_PLAYER).into();
        itertools::assert_equal(
            sorted(chess.get_attack_threats(e1, &player1)),
            [c3, e3, g3, c1, g1],
        );

        // create two threatening queens closer to king that two of the other five
        chess.board[f1] = Piece::create_queen(SECOND_PLAYER).into();
        chess.board[e2] = Piece::create_queen(SECOND_PLAYER).into();
        itertools::assert_equal(
            sorted(chess.get_attack_threats(e1, &player1)),
            [c3, g3, e2, c1, f1],
        );
    }

    #[test]
    fn test_find_pieces_positions() {
        let [a1, b1, c1, d1, e1, f1, g1, h1]: [_; 8] = row_indices(7).try_into().unwrap();
        let [a2, b2, c2, d2, e2, f2, g2, h2]: [_; 8] = row_indices(6).try_into().unwrap();
        let [a7, b7, c7, d7, e7, f7, g7, h7]: [_; 8] = row_indices(1).try_into().unwrap();
        let [a8, b8, c8, d8, e8, f8, g8, h8]: [_; 8] = row_indices(0).try_into().unwrap();
        let mut chess = Chess::new();

        itertools::assert_equal(
            chess.find_pieces_positions(FIRST_PLAYER),
            [
                a2, b2, c2, d2, e2, f2, g2, h2, a1, b1, c1, d1, e1, f1, g1, h1,
            ],
        );
        itertools::assert_equal(
            chess.find_pieces_positions(SECOND_PLAYER),
            [
                a8, b8, c8, d8, e8, f8, g8, h8, a7, b7, c7, d7, e7, f7, g7, h7,
            ],
        );

        for row in 0..8 {
            for col in 0..8 {
                chess.board[GridIndex::new(row, col)].take();
            }
        }
        itertools::assert_equal(chess.find_pieces_positions(FIRST_PLAYER), []);
        itertools::assert_equal(chess.find_pieces_positions(SECOND_PLAYER), []);
    }

    #[test]
    fn test_update_state_without_checks_just_switches_turns() {
        let mut chess = Chess::new();

        assert_eq!(chess.state, GameState::Turn(FIRST_PLAYER));
        assert!(!chess.is_in_check(FIRST_PLAYER));
        assert!(!chess.is_in_check(SECOND_PLAYER));
        assert_eq!(
            chess.update_state().unwrap(),
            GameState::Turn(SECOND_PLAYER)
        );
        assert!(!chess.is_in_check(FIRST_PLAYER));
        assert!(!chess.is_in_check(SECOND_PLAYER));
        assert_eq!(chess.update_state().unwrap(), GameState::Turn(FIRST_PLAYER));
        assert!(!chess.is_in_check(FIRST_PLAYER));
        assert!(!chess.is_in_check(SECOND_PLAYER));
    }

    #[test]
    fn test_update_state_sets_winner() {
        let a1 = GridIndex::new(7, 0);
        let h1 = GridIndex::new(7, 7);
        let a7 = GridIndex::new(1, 0);
        let h8 = GridIndex::new(0, 7);
        let mut chess = create_board_kings_and_rooks_only();

        // rooks on h8 and a7 is checkmate
        chess.move_piece(h1, h8).unwrap();
        chess.move_piece(a1, a7).unwrap();
        assert_eq!(
            chess.update_state().unwrap(),
            GameState::Finished(FinishedState::Win(FIRST_PLAYER))
        );
    }

    #[test]
    fn test_update_state_sets_draw() {
        let a1 = GridIndex::new(7, 0);
        let g1 = GridIndex::new(7, 6);
        let h1 = GridIndex::new(7, 7);
        let a7 = GridIndex::new(1, 0);
        let a8 = GridIndex::new(0, 0);
        let e8 = GridIndex::new(0, 4);
        let h8 = GridIndex::new(0, 7);
        let mut chess = create_board_kings_and_rooks_only();

        // move king to the corner and delete all other black pieces
        chess.board[a8].take();
        chess.move_piece(e8, h8).unwrap();
        chess.update_king_position(SECOND_PLAYER, h8);
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
        let [_, _, _, _, e2, _, _, _]: [_; 8] = row_indices(6).try_into().unwrap();
        let [_, _, _, _, e7, _, _, _]: [_; 8] = row_indices(1).try_into().unwrap();
        let e3 = GridIndex::new(5, 4);
        let e4 = GridIndex::new(4, 4);
        let e5 = GridIndex::new(3, 4);
        let mut chess = Chess::new();

        // cannot update finished game
        chess.set_draw();
        assert_eq!(
            chess
                .update(FIRST_PLAYER, TurnData::new(e2, e4))
                .unwrap_err(),
            GameError::GameIsFinished
        );

        // reset state
        chess.set_state(GameState::Turn(FIRST_PLAYER));

        // second player cannot call update while its first player's turn
        assert_eq!(
            chess
                .update(SECOND_PLAYER, TurnData::new(e7, e5))
                .unwrap_err(),
            GameError::not_your_turn(FIRST_PLAYER, SECOND_PLAYER)
        );

        // player cannot specify empty cell as 'to'
        assert_eq!(
            chess
                .update(FIRST_PLAYER, TurnData::new(e3, e5))
                .unwrap_err(),
            GameError::cell_is_empty(e3.row(), e3.col())
        );

        // player cannot move another player's piece
        assert_eq!(
            chess
                .update(FIRST_PLAYER, TurnData::new(e7, e5))
                .unwrap_err(),
            GameError::unauthorized_move(SECOND_PLAYER, FIRST_PLAYER)
        );

        // invalid move
        assert_eq!(
            chess
                .update(FIRST_PLAYER, TurnData::new(e2, e5))
                .unwrap_err(),
            GameError::invalid_move(format!("unable to move {} to {}", e2, e5))
        );
    }
}
