use generic_array::typenum::Unsigned;
use generic_array::{ArrayLength, GenericArray};
use std::cmp::Ordering;
use std::marker::PhantomData;
use std::ops::{Add, Deref, Sub};

pub trait WithMaxValue {
    type MaxValue: ArrayLength;

    fn max() -> Self
    where
        Self: Sized + From<usize>,
    {
        (Self::MaxValue::to_usize() - 1).into()
    }
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
    Row: PartialOrd + From<usize> + WithMaxValue,
    Col: PartialOrd + From<usize> + WithMaxValue,
{
    pub fn is_valid(&self) -> bool {
        self.row >= Row::from(0)
            && self.row <= Row::max()
            && self.col >= Col::from(0)
            && self.col <= Col::max()
    }
}

impl<Row, Col> GridIndex<Row, Col>
where
    Row: Copy + PartialOrd + Sub<usize, Output = Row>,
    Col: Copy + PartialOrd + Sub<usize, Output = Col>,
{
    pub fn is_adjacent(&self, other: &GridIndex<Row, Col>) -> bool {
        let vertically_adjacent = match self.row.partial_cmp(&other.row) {
            Some(Ordering::Equal) => true,
            Some(Ordering::Less) => *self == GridIndex::new(other.row - 1, other.col),
            Some(Ordering::Greater) => *other == GridIndex::new(self.row - 1, self.col),
            None => false,
        };
        let horizontally_adjacent = match self.col.partial_cmp(&other.col) {
            Some(Ordering::Equal) => true,
            Some(Ordering::Less) => *self == GridIndex::new(other.row, other.col - 1),
            Some(Ordering::Greater) => *other == GridIndex::new(self.row, self.col - 1),
            None => false,
        };
        vertically_adjacent && horizontally_adjacent
    }
}

impl<Row, Col> GridIndex<Row, Col>
where
    Row: Copy
        + PartialOrd
        + From<usize>
        + Add<usize, Output = Row>
        + Sub<usize, Output = Row>
        + WithMaxValue,
    Col: Copy
        + PartialOrd
        + From<usize>
        + Add<usize, Output = Col>
        + Sub<usize, Output = Col>
        + WithMaxValue,
{
    pub fn move_right(&self, n: usize) -> Option<Self> {
        let moved = GridIndex::new(self.row, self.col + n);
        if moved.is_valid() {
            Some(moved)
        } else {
            None
        }
    }

    pub fn move_left(&self, n: usize) -> Option<Self> {
        if self.col >= n.into() {
            Some(GridIndex::new(self.row, self.col - n))
        } else {
            None
        }
    }

    pub fn move_up(&self, n: usize) -> Option<Self> {
        if self.row >= n.into() {
            Some(GridIndex::new(self.row - n, self.col))
        } else {
            None
        }
    }

    pub fn move_down(&self, n: usize) -> Option<Self> {
        let moved = GridIndex::new(self.row + n, self.col);
        if moved.is_valid() {
            Some(moved)
        } else {
            None
        }
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

    pub fn get_ref(&self, idx: GridIndex<Row, Col>) -> &T {
        &self.contents[idx.get_row()][idx.get_col()]
    }
}

impl<T, Row, Col> Grid<T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize> + Sub<usize> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize> + Sub<usize> + WithMaxValue,
{
    pub fn right_iter(&self, pos: GridIndex<Row, Col>) -> RightGridIterator<T, Row, Col> {
        RightGridIterator {
            current: pos,
            grid: &self,
        }
    }

    pub fn left_iter(&self, pos: GridIndex<Row, Col>) -> LeftGridIterator<T, Row, Col> {
        LeftGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    pub fn top_iter(&self, pos: GridIndex<Row, Col>) -> TopGridIterator<T, Row, Col> {
        TopGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    pub fn bottom_iter(&self, pos: GridIndex<Row, Col>) -> BottomGridIterator<T, Row, Col> {
        BottomGridIterator {
            current: pos,
            grid: &self,
        }
    }

    pub fn top_left_iter(&self, pos: GridIndex<Row, Col>) -> TopLeftGridIterator<T, Row, Col> {
        TopLeftGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    pub fn top_right_iter(&self, pos: GridIndex<Row, Col>) -> TopRightGridIterator<T, Row, Col> {
        TopRightGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    pub fn bottom_right_iter(
        &self,
        pos: GridIndex<Row, Col>,
    ) -> BottomRightGridIterator<T, Row, Col> {
        BottomRightGridIterator {
            current: pos,
            grid: &self,
        }
    }

    pub fn bottom_left_iter(
        &self,
        pos: GridIndex<Row, Col>,
    ) -> BottomLeftGridIterator<T, Row, Col> {
        BottomLeftGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }
}

pub struct RightGridIterator<'a, T, Row: WithMaxValue, Col: WithMaxValue> {
    current: GridIndex<Row, Col>, // no need for an Option as we're only incrementing
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for RightGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Col> + WithMaxValue,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_valid() {
            let old_current = self.current;
            self.current = GridIndex::new(self.current.row, self.current.col + 1);
            return Some(self.grid.get_ref(old_current));
        }
        None
    }
}

pub struct LeftGridIterator<'a, T, Row: WithMaxValue, Col: WithMaxValue> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for LeftGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Col> + WithMaxValue,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.is_valid() {
                let old_current = current;
                if current.row == 0.into() || current.col == 0.into() {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row, current.col - 1));
                }
                return Some(self.grid.get_ref(old_current));
            }
        }
        None
    }
}

pub struct TopGridIterator<'a, T, Row: WithMaxValue, Col: WithMaxValue> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for TopGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Row> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + WithMaxValue,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.is_valid() {
                let old_current = current;
                if current.row == 0.into() || current.col == 0.into() {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row - 1, current.col));
                }
                return Some(self.grid.get_ref(old_current));
            }
        }
        None
    }
}

pub struct BottomGridIterator<'a, T, Row: WithMaxValue, Col: WithMaxValue> {
    current: GridIndex<Row, Col>, // no need for an Option as we're only incrementing
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for BottomGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Row> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + WithMaxValue,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_valid() {
            let old_current = self.current;
            self.current = GridIndex::new(self.current.row + 1, self.current.col);
            return Some(self.grid.get_ref(old_current));
        }
        None
    }
}

pub struct TopLeftGridIterator<'a, T, Row: WithMaxValue, Col: WithMaxValue> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for TopLeftGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Row> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Col> + WithMaxValue,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.is_valid() {
                let old_current = current;
                if current.row == 0.into() || current.col == 0.into() {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row - 1, current.col - 1));
                }
                return Some(self.grid.get_ref(old_current));
            }
        }
        None
    }
}

pub struct TopRightGridIterator<'a, T, Row: WithMaxValue, Col: WithMaxValue> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for TopRightGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Row> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Col> + WithMaxValue,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.is_valid() {
                let old_current = current;
                if current.row == 0.into() {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row - 1, current.col + 1));
                }
                return Some(self.grid.get_ref(old_current));
            }
        }
        None
    }
}

pub struct BottomRightGridIterator<'a, T, Row: WithMaxValue, Col: WithMaxValue> {
    current: GridIndex<Row, Col>, // no need for an Option as we're only incrementing
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for BottomRightGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Row> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Col> + WithMaxValue,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_valid() {
            let old_current = self.current;
            self.current = GridIndex::new(self.current.row + 1, self.current.col + 1);
            return Some(self.grid.get_ref(old_current));
        }
        None
    }
}

pub struct BottomLeftGridIterator<'a, T, Row: WithMaxValue, Col: WithMaxValue> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for BottomLeftGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Row> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Col> + WithMaxValue,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.is_valid() {
                let old_current = current;
                if current.col == 0.into() {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row + 1, current.col - 1));
                }
                return Some(self.grid.get_ref(old_current));
            }
        }
        None
    }
}

pub trait WithGridIndex<Row, Col> {
    fn get_index(&self) -> Option<GridIndex<Row, Col>>;

    fn indexed(self) -> IndexedGridIterator<Row, Col, Self>
    where
        Self: Sized,
    {
        IndexedGridIterator {
            it: self,
            phantom_data: Default::default(),
        }
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for RightGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithMaxValue,
    Col: Copy + WithMaxValue,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        Some(self.current)
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for LeftGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithMaxValue,
    Col: Copy + WithMaxValue,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for TopGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithMaxValue,
    Col: Copy + WithMaxValue,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for BottomGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithMaxValue,
    Col: Copy + WithMaxValue,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        Some(self.current)
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for TopLeftGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithMaxValue,
    Col: Copy + WithMaxValue,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for TopRightGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithMaxValue,
    Col: Copy + WithMaxValue,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for BottomRightGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithMaxValue,
    Col: Copy + WithMaxValue,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        Some(self.current)
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for BottomLeftGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithMaxValue,
    Col: Copy + WithMaxValue,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

pub struct IndexedGridIterator<Row, Col, It> {
    it: It,
    phantom_data: PhantomData<(Row, Col)>,
}

impl<Row, Col, It> Iterator for IndexedGridIterator<Row, Col, It>
where
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize> + WithMaxValue,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize> + WithMaxValue,
    It: Iterator + WithGridIndex<Row, Col>,
{
    type Item = (GridIndex<Row, Col>, It::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.it.get_index();
        match self.it.next() {
            // unwrap() here is ok if next() returned Some()
            Some(item) => Some((index.unwrap(), item)),
            None => None,
        }
    }
}
