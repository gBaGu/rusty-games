use prost::Message;

use crate::game::encoding::{ProtobufResult, ToProtobuf};
use crate::game::grid::GridIndex;
use crate::game::PlayerId;
use crate::proto;

#[derive(Debug, PartialEq)]
pub enum MoveType {
    LeftCastling,
    RightCastling,
    KingMove,
    RookMove,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Team {
    Black,
    White,
}

impl Team {
    pub fn get_king_initial_position(&self) -> GridIndex {
        match self {
            Team::White => GridIndex::new(7, 4),
            Team::Black => GridIndex::new(0, 4),
        }
    }

    pub fn get_left_rook_initial_position(&self) -> GridIndex {
        match self {
            Team::White => GridIndex::new(7, 0),
            Team::Black => GridIndex::new(0, 0),
        }
    }

    pub fn get_right_rook_initial_position(&self) -> GridIndex {
        match self {
            Team::White => GridIndex::new(7, 7),
            Team::Black => GridIndex::new(0, 7),
        }
    }

    pub fn get_pawn_initial_row(&self) -> usize {
        match self {
            Team::White => 6,
            Team::Black => 1,
        }
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Piece {
    pub kind: PieceKind,
    pub owner: PlayerId,
}

impl ToProtobuf for Piece {
    fn to_protobuf(self) -> ProtobufResult<Vec<u8>> {
        Ok(<Self as Into<proto::ChessPiece>>::into(self).encode_to_vec())
    }
}

impl Piece {
    pub fn create_pawn(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Pawn,
            owner,
        }
    }

    pub fn create_bishop(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Bishop,
            owner,
        }
    }

    pub fn create_knight(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Knight,
            owner,
        }
    }

    pub fn create_rook(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Rook,
            owner,
        }
    }

    pub fn create_queen(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::Queen,
            owner,
        }
    }

    pub fn create_king(owner: PlayerId) -> Self {
        Self {
            kind: PieceKind::King,
            owner,
        }
    }

    #[allow(dead_code)]
    pub fn is_pawn(&self) -> bool {
        self.kind == PieceKind::Pawn
    }

    #[allow(dead_code)]
    pub fn is_bishop(&self) -> bool {
        self.kind == PieceKind::Bishop
    }

    #[allow(dead_code)]
    pub fn is_knight(&self) -> bool {
        self.kind == PieceKind::Knight
    }

    pub fn is_rook(&self) -> bool {
        self.kind == PieceKind::Rook
    }

    #[allow(dead_code)]
    pub fn is_queen(&self) -> bool {
        self.kind == PieceKind::Queen
    }

    pub fn is_king(&self) -> bool {
        self.kind == PieceKind::King
    }

    pub fn is_enemy(&self, player: PlayerId) -> bool {
        self.owner != player
    }
}
