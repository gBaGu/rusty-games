tonic::include_proto!("game");

use crate::game::chess;
use crate::game::game;

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("game_descriptor");

impl From<game::GameState> for GameState {
    fn from(value: game::GameState) -> Self {
        match value {
            game::GameState::Turn(id) => Self {
                next_player_id: Some(id),
                ..Default::default()
            },
            game::GameState::Finished(game::FinishedState::Win(id)) => Self {
                winner: Some(id),
                ..Default::default()
            },
            game::GameState::Finished(game::FinishedState::Draw) => Self::default(),
        }
    }
}

impl From<chess::types::PieceKind> for ChessPieceKind {
    fn from(value: chess::types::PieceKind) -> Self {
        match value {
            chess::types::PieceKind::Pawn => ChessPieceKind::PieceKindPawn,
            chess::types::PieceKind::Bishop => ChessPieceKind::PieceKindBishop,
            chess::types::PieceKind::Knight => ChessPieceKind::PieceKindKnight,
            chess::types::PieceKind::Rook => ChessPieceKind::PieceKindRook,
            chess::types::PieceKind::Queen => ChessPieceKind::PieceKindQueen,
            chess::types::PieceKind::King => ChessPieceKind::PieceKindKing,
        }
    }
}

impl From<chess::types::Piece> for ChessPiece {
    fn from(value: chess::types::Piece) -> Self {
        let kind: ChessPieceKind = value.kind.into();
        Self {
            kind: kind.into(),
            owner: value.owner,
        }
    }
}
