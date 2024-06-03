use prost::Message;

use crate::game::chess::index::{Col, Index, Row};
use crate::game::encoding::ToProtobuf;
use crate::game::grid::WithLength;
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
    pub fn get_king_initial_position(&self) -> Index {
        match self {
            Team::White => Index::new(Row::max(), Col(4)),
            Team::Black => Index::new(Row(0), Col(4)),
        }
    }

    pub fn get_left_rook_initial_position(&self) -> Index {
        match self {
            Team::White => Index::new(Row::max(), Col(0)),
            Team::Black => Index::new(Row(0), Col(0)),
        }
    }

    pub fn get_right_rook_initial_position(&self) -> Index {
        match self {
            Team::White => Index::new(Row::max(), Col::max()),
            Team::Black => Index::new(Row(0), Col::max()),
        }
    }

    pub fn get_pawn_initial_row(&self) -> Row {
        match self {
            Team::White => Row::max() - 1,
            Team::Black => Row(1),
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
    fn to_protobuf(self) -> Vec<u8> {
        <Self as Into<proto::ChessPiece>>::into(self).encode_to_vec()
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
