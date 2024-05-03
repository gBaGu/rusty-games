use std::ops::{Add, Sub};

use crate::game::grid::{GridIndex, WithMaxValue};

pub type Index = GridIndex<Row, Col>;

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