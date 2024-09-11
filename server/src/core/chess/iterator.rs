use std::iter::{Iterator, Scan};

use generic_array::ArrayLength;

use super::types::Piece;
use crate::core::grid::{Grid, GridIndex, WithGridIndex};
use crate::core::BoardCell;

type IndexedCell<'a> = (GridIndex, &'a BoardCell<Piece>);

// iterator helper
pub fn while_empty<'a, I>(
    it: I,
) -> Scan<I, bool, impl FnMut(&mut bool, I::Item) -> Option<I::Item> + 'a>
where
    I: Iterator<Item = IndexedCell<'a>>,
{
    it.scan(false, |encountered, elem| {
        if *encountered {
            return None;
        }
        *encountered = elem.1.is_some();
        Some(elem)
    })
}

pub trait GridExt {
    fn right_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell>;
    fn left_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell>;
    fn up_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell>;
    fn down_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell>;
    fn up_right_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell>;
    fn down_right_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell>;
    fn down_left_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell>;
    fn up_left_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell>;
    fn knight_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell>;
}

impl<R: ArrayLength, C: ArrayLength> GridExt for Grid<BoardCell<Piece>, R, C> {
    fn right_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell> {
        self.right_iter(pos).indexed().skip(1)
    }

    fn left_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell> {
        self.left_iter(pos).indexed().skip(1)
    }

    fn up_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell> {
        self.top_iter(pos).indexed().skip(1)
    }

    fn down_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell> {
        self.bottom_iter(pos).indexed().skip(1)
    }

    fn up_right_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell> {
        self.top_right_iter(pos).indexed().skip(1)
    }

    fn down_right_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell> {
        self.bottom_right_iter(pos).indexed().skip(1)
    }

    fn down_left_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell> {
        self.bottom_left_iter(pos).indexed().skip(1)
    }

    fn up_left_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell> {
        self.top_left_iter(pos).indexed().skip(1)
    }

    fn knight_move_iter(&self, pos: GridIndex) -> impl Iterator<Item = IndexedCell> {
        let up = self.top_iter(pos).indexed().nth(2).into_iter();
        let down = self.bottom_iter(pos).indexed().nth(2).into_iter();
        let right = self.right_iter(pos).indexed().nth(2).into_iter();
        let left = self.left_iter(pos).indexed().nth(2).into_iter();
        up.chain(down)
            .flat_map(|(pos, _)| {
                [
                    self.right_iter(pos).indexed().nth(1),
                    self.left_iter(pos).indexed().nth(1),
                ]
                .into_iter()
                .flatten()
            })
            .chain(right.chain(left).flat_map(|(pos, _)| {
                [
                    self.top_iter(pos).indexed().nth(1),
                    self.bottom_iter(pos).indexed().nth(1),
                ]
                .into_iter()
                .flatten()
            }))
    }
}
