tonic::include_proto!("game");

use std::num::TryFromIntError;

use crate::game;
use crate::game::chess;

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("game_descriptor");

impl game_session_request::Request {
    pub fn name(&self) -> String {
        match self {
            Self::Init(_) => "Init".into(),
            Self::TurnData(_) => "TurnData".into(),
        }
    }
}

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

impl TryFrom<game::grid::GridIndex> for Position {
    type Error = TryFromIntError;

    fn try_from(value: game::grid::GridIndex) -> Result<Self, Self::Error> {
        Ok(Self {
            row: value.row().try_into()?,
            col: value.col().try_into()?,
        })
    }
}

impl TryFrom<chess::turn_data::TurnData> for PositionPair {
    type Error = TryFromIntError;

    fn try_from(value: chess::turn_data::TurnData) -> Result<Self, Self::Error> {
        Ok(Self {
            first: Some(value.from.try_into()?),
            second: Some(value.to.try_into()?),
        })
    }
}
