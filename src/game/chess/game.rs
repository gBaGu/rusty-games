use std::cmp::Ordering;
use std::collections::HashMap;
use std::iter::Scan;

use crate::game::chess::index::{Col, Index, Row};
use crate::game::chess::turn_data::TurnData;
use crate::game::chess::types::{MoveType, Piece, PieceKind, Team};
use crate::game::error::GameError;
use crate::game::game::{FinishedState, Game, GameResult, GameState};
use crate::game::grid::{Grid, WithGridIndex, WithLength};
use crate::game::player_pool::{PlayerId, PlayerPool, WithPlayerId};

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
        let [id1, id2]: [_; 2] = players
            .try_into()
            .map_err(|_| GameError::invalid_players_number(2, players.len()))?;
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
        let enemy = *self.get_enemy_player()?;
        self.update_check(&enemy);
        self.update_check(&player);

        self.update_state()
    }

    fn state(&self) -> GameState {
        self.state
    }

    fn set_state(&mut self, state: GameState) {
        self.state = state;
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

    fn get_cell(&self, position: Index) -> &Cell {
        self.board.get_ref(position)
    }

    fn get_cell_mut(&mut self, position: Index) -> &mut Cell {
        self.board.get_mut_ref(position)
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
            // castling is disabled once king has moved
            state.castle_options = CastleOptions::none();
        }
    }

    fn update_check(&mut self, player: &Player) {
        if let Some(king_pos) = self.get_king_position(player.id) {
            let threats = self.get_attack_threats(king_pos, player);
            if let Some(state) = self.additional_state.get_mut(&player.id) {
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
        if let Some(threats) = self.additional_state.get(&id).map(|state| &state.check) {
            return !threats.is_empty();
        }
        false
    }

    fn get_king_position(&self, id: PlayerId) -> Option<Index> {
        self.additional_state.get(&id).map(|state| state.king_pos)
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
                            Team::White => enemy_pos.row() > pos.row(),
                            Team::Black => enemy_pos.row() < pos.row(),
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
            if cell.filter(|p| !p.is_enemy(piece.owner)).is_some() {
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
        res.retain(|&index| {
            let backup = match self.move_piece(pos, index) {
                Ok(cell) => cell,
                Err(_) => return false,
            };
            let king_safe = self.get_attack_threats(king_pos, &player).is_empty();
            if let Err(_) = self.move_piece(index, pos) {
                return false;
            }
            *self.get_cell_mut(index) = backup;
            king_safe
        });
        Ok(res)
    }

    fn switch_player(&mut self) -> GameResult<GameState> {
        let next_player = self.players.next().ok_or(GameError::PlayerPoolCorrupted)?;
        self.state = GameState::Turn(next_player.id);
        Ok(self.state)
    }

    fn update_state(&mut self) -> GameResult<GameState> {
        let enemy = *self.get_enemy_player()?;
        let mut enemie_pieces = vec![];
        for row in 0..=Row::max().0 {
            for col in 0..=Col::max().0 {
                if let Some(piece) = self.get_cell(Index::new(Row(row), Col(col))) {
                    if !piece.is_enemy(enemy.id) {
                        enemie_pieces.push(Index::new(Row(row), Col(col)));
                    }
                }
            }
        }
        if enemie_pieces.into_iter().all(|index| {
            if let Ok(moves) = self.get_moves(index) {
                return moves.is_empty();
            }
            true
        }) {
            if self.is_in_check(enemy.id) {
                let current_id = self.get_current_player()?.id;
                return Ok(self.set_winner(current_id));
            } else {
                // stalemate
                self.state = GameState::Finished(FinishedState::Draw);
            }
        }

        self.switch_player()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_creation() {
        let (player1, player2) = (1u64, 2u64);
        // Chess::new with less than players should fail
        assert_eq!(
            Chess::new(&[]).unwrap_err(),
            GameError::invalid_players_number(2, 0)
        );
        assert_eq!(
            Chess::new(&[player1]).unwrap_err(),
            GameError::invalid_players_number(2, 1)
        );
        // Chess::new with more than players should fail
        assert_eq!(
            Chess::new(&[player1, player2, 3]).unwrap_err(),
            GameError::invalid_players_number(2, 3)
        );
        assert_eq!(
            Chess::new(&[player1, player2, 3, 9]).unwrap_err(),
            GameError::invalid_players_number(2, 4)
        );
        // Chess::new with duplicated player id should fail
        assert_eq!(
            Chess::new(&[player1, player1]).unwrap_err(),
            GameError::DuplicatePlayerId
        );

        let mut chess = Chess::new(&[player1, player2]).unwrap();
        assert_eq!(chess.get_current_player().unwrap().id, player1);
        assert_eq!(chess.get_current_player().unwrap().team, Team::White);
        assert_eq!(chess.get_enemy_player().unwrap().id, player2);
        assert_eq!(chess.get_enemy_player().unwrap().team, Team::Black);
        assert_eq!(chess.state(), GameState::Turn(player1));

        // check that initial board is correct
        let (p1_backline_expected, p2_backline_expected): (Vec<_>, Vec<_>) = [
            (Piece::create_rook(player1), Piece::create_rook(player2)),
            (Piece::create_knight(player1), Piece::create_knight(player2)),
            (Piece::create_bishop(player1), Piece::create_bishop(player2)),
            (Piece::create_queen(player1), Piece::create_queen(player2)),
            (Piece::create_king(player1), Piece::create_king(player2)),
            (Piece::create_bishop(player1), Piece::create_bishop(player2)),
            (Piece::create_knight(player1), Piece::create_knight(player2)),
            (Piece::create_rook(player1), Piece::create_rook(player2)),
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
            std::iter::repeat(Piece::create_pawn(player1)).take(8),
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
            std::iter::repeat(Piece::create_pawn(player2)).take(8),
        );

        // check additional state
        assert_eq!(chess.is_in_check(player1), false);
        assert_eq!(
            chess.get_king_position(player1).unwrap(),
            Team::White.get_king_initial_position()
        );
        assert_eq!(
            chess.additional_state.get(&player1).unwrap().castle_options,
            CastleOptions::all()
        );
        assert_eq!(chess.is_in_check(player2), false);
        assert_eq!(
            chess.get_king_position(player2).unwrap(),
            Team::Black.get_king_initial_position()
        );
        assert_eq!(
            chess.additional_state.get(&player2).unwrap().castle_options,
            CastleOptions::all()
        );
    }
}
