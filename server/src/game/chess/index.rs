use std::fmt::{Display, Formatter, Write};
use std::ops::{Add, Sub};

use crate::game::grid::{GridIndex, WithLength};

pub type Index = GridIndex<Row, Col>;

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Row(pub usize);

impl WithLength for Row {
    type Length = generic_array::typenum::U8;
}

impl Display for Row {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0 => f.write_char('8'),
            1 => f.write_char('7'),
            2 => f.write_char('6'),
            3 => f.write_char('5'),
            4 => f.write_char('4'),
            5 => f.write_char('3'),
            6 => f.write_char('2'),
            7 => f.write_char('1'),
            _ => f.write_str("INVALID"),
        }
    }
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

#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct Col(pub usize);

impl Display for Col {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            0 => f.write_char('a'),
            1 => f.write_char('b'),
            2 => f.write_char('c'),
            3 => f.write_char('d'),
            4 => f.write_char('e'),
            5 => f.write_char('f'),
            6 => f.write_char('g'),
            7 => f.write_char('h'),
            _ => f.write_str("INVALID"),
        }
    }
}

impl WithLength for Col {
    type Length = generic_array::typenum::U8;
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