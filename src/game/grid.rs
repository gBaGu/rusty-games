use generic_array::{ArrayLength, GenericArray};
use std::ops::Deref;


pub trait WithMaxValue {
    type MaxValue: ArrayLength;
}


// Struct used to mutably access items in Grid
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridIndex<Row, Col> {
    row: Row,
    col: Col,
}

impl<Row, Col> GridIndex<Row, Col> {
    pub fn new(row: Row, col: Col) -> Self {
        Self { row, col }
    }
}

impl<Row, Col> GridIndex<Row, Col>
where
    Row: Copy + Into<usize>,
    Col: Copy + Into<usize>,
{
    pub fn get_col(&self) -> usize {
        self.col.into()
    }

    pub fn get_row(&self) -> usize {
        self.row.into()
    }
}


// Two-dimensional fixed-length array that stores values and allows to mutate them
#[derive(Debug)]
pub struct Grid<T, Row: WithMaxValue, Col: WithMaxValue> {
    contents: GenericArray<GenericArray<T, Col::MaxValue>, Row::MaxValue>,
}

impl<T: Default, Row: WithMaxValue, Col: WithMaxValue> Default for Grid<T, Row, Col> {
    fn default() -> Self {
        Self {
            contents: Default::default(),
        }
    }
}

impl<T: Default, Row: WithMaxValue, Col: WithMaxValue> Deref for Grid<T, Row, Col> {
    type Target = [GenericArray<T, Col::MaxValue>];

    fn deref(&self) -> &Self::Target {
        self.contents.as_slice()
    }
}

impl<T, Row, Col> Grid<T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + WithMaxValue,
    Col: Copy + Into<usize> + WithMaxValue,
{
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn get_mut_ref(&mut self, idx: GridIndex<Row, Col>) -> &mut T {
        &mut self.contents[idx.get_row()][idx.get_col()]
    }
}
