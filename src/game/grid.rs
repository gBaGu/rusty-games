use generic_array::typenum::Unsigned;
use generic_array::{ArrayLength, GenericArray};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::ops::{Add, Deref, Sub};

/// Needed for types that represent `Row` or `Col` in the [`Grid`] generic parameters.
pub trait WithLength {
    /// Used to create GenericArray for the [`Grid`].
    type Length: ArrayLength;

    /// Returns a maximum value that can be used as an index in the respective [`GenericArray`].
    fn max() -> Self
    where
        Self: Sized + From<usize>,
    {
        (Self::Length::to_usize() - 1).into()
    }
}

/// Index struct to access elements in the [`Grid`].
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct GridIndex<Row, Col> {
    row: Row,
    col: Col,
}

impl<Row, Col> GridIndex<Row, Col> {
    /// Constructs a new [`GridIndex`].
    pub fn new(row: Row, col: Col) -> Self {
        Self { row, col }
    }
}

impl<Row: Display, Col: Display> Display for GridIndex<Row, Col> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.col, self.row)
    }
}

impl<Row, Col> GridIndex<Row, Col>
where
    Row: PartialOrd + From<usize> + WithLength,
    Col: PartialOrd + From<usize> + WithLength,
{
    /// Returns `true` if `self.row` and `self.col` are valid to use when accessing the [`Grid`] contents.
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
    /// Compares `self.row` to `other.row` and `self.col` to `other.col`.
    /// Returns `true` if the difference in both cases is less than or equal to 1.
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
    Row: Copy + PartialOrd + From<usize> + WithLength,
    Col: Copy + PartialOrd + From<usize> + WithLength,
{
    /// Returns new instance of [`GridIndex`] with the `col` value increased by `n` or
    /// [`None`] if this instance is not valid.
    pub fn move_right(&self, n: usize) -> Option<Self>
    where
        Col: Add<usize, Output = Col>,
    {
        let moved = GridIndex::new(self.row, self.col + n);
        if moved.is_valid() {
            Some(moved)
        } else {
            None
        }
    }

    /// Returns new instance of [`GridIndex`] with the `col` value decreased by `n` or
    /// [`None`] if this instance is not valid.
    pub fn move_left(&self, n: usize) -> Option<Self>
    where
        Col: Sub<usize, Output = Col>,
    {
        if self.col >= n.into() {
            Some(GridIndex::new(self.row, self.col - n))
        } else {
            None
        }
    }

    /// Returns new instance of [`GridIndex`] with the `row` value decreased by `n` or
    /// [`None`] if this instance is not valid.
    pub fn move_up(&self, n: usize) -> Option<Self>
    where
        Row: Sub<usize, Output = Row>,
    {
        if self.row >= n.into() {
            Some(GridIndex::new(self.row - n, self.col))
        } else {
            None
        }
    }

    /// Returns new instance of [`GridIndex`] with the `row` value increased by `n` or
    /// [`None`] if this instance is not valid.
    pub fn move_down(&self, n: usize) -> Option<Self>
    where
        Row: Add<usize, Output = Row>,
    {
        let moved = GridIndex::new(self.row + n, self.col);
        if moved.is_valid() {
            Some(moved)
        } else {
            None
        }
    }
}

impl<Row: Copy, Col: Copy> GridIndex<Row, Col> {
    /// Returns value of `self.col`
    pub fn col(&self) -> Col {
        self.col
    }

    /// Returns value of `self.row`
    pub fn row(&self) -> Row {
        self.row
    }
}

/// Two-dimensional fixed-length array that stores values and allows to mutate them.
/// Length of array is defined by generic parameters `Row` and `Col`.
#[derive(Debug)]
pub struct Grid<T, Row: WithLength, Col: WithLength> {
    contents: GenericArray<GenericArray<T, Col::Length>, Row::Length>,
}

impl<T: Default, Row: WithLength, Col: WithLength> Default for Grid<T, Row, Col> {
    fn default() -> Self {
        Self {
            contents: Default::default(),
        }
    }
}

impl<T, Row: WithLength, Col: WithLength> Deref for Grid<T, Row, Col> {
    type Target = [GenericArray<T, Col::Length>];

    fn deref(&self) -> &Self::Target {
        self.contents.as_slice()
    }
}

impl<T, Row, Col> Grid<T, Row, Col>
where
    Row: Copy + Into<usize> + WithLength,
    Col: Copy + Into<usize> + WithLength,
{
    /// Returns a mutable reference to a single element in a [`Grid`] located at `idx`.
    ///
    /// # Panics
    ///
    /// May panic if `idx` is out of bounds.
    pub fn get_mut_ref(&mut self, idx: GridIndex<Row, Col>) -> &mut T {
        &mut self.contents[idx.row().into()][idx.col().into()]
    }

    /// Returns an immutable reference to a single element in a [`Grid`] located at `idx`.
    ///
    /// # Panics
    ///
    /// May panic if `idx` is out of bounds.
    pub fn get_ref(&self, idx: GridIndex<Row, Col>) -> &T {
        &self.contents[idx.row().into()][idx.col().into()]
    }
}

impl<T, Row, Col> Grid<T, Row, Col>
where
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize> + Sub<usize> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize> + Sub<usize> + WithLength,
{
    /// Returns an iterator with rightwards direction that starts with a `pos`.
    pub fn right_iter(&self, pos: GridIndex<Row, Col>) -> RightGridIterator<T, Row, Col> {
        RightGridIterator {
            current: pos,
            grid: &self,
        }
    }

    /// Returns an iterator with leftwards direction that starts with a `pos`.
    pub fn left_iter(&self, pos: GridIndex<Row, Col>) -> LeftGridIterator<T, Row, Col> {
        LeftGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    /// Returns an iterator with upwards direction that starts with a `pos`.
    pub fn top_iter(&self, pos: GridIndex<Row, Col>) -> TopGridIterator<T, Row, Col> {
        TopGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    /// Returns an iterator with downwards direction that starts with a `pos`.
    pub fn bottom_iter(&self, pos: GridIndex<Row, Col>) -> BottomGridIterator<T, Row, Col> {
        BottomGridIterator {
            current: pos,
            grid: &self,
        }
    }

    /// Returns a diagonal iterator with top-left direction that starts with a `pos`.
    pub fn top_left_iter(&self, pos: GridIndex<Row, Col>) -> TopLeftGridIterator<T, Row, Col> {
        TopLeftGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    /// Returns a diagonal iterator with top-right direction that starts with a `pos`.
    pub fn top_right_iter(&self, pos: GridIndex<Row, Col>) -> TopRightGridIterator<T, Row, Col> {
        TopRightGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    /// Returns a diagonal iterator with bottom-right direction that starts with a `pos`.
    pub fn bottom_right_iter(
        &self,
        pos: GridIndex<Row, Col>,
    ) -> BottomRightGridIterator<T, Row, Col> {
        BottomRightGridIterator {
            current: pos,
            grid: &self,
        }
    }

    /// Returns a diagonal iterator with bottom-left direction that starts with a `pos`.
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

/// An iterator with rightwards direction.
/// On each step it's incrementing `col` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct RightGridIterator<'a, T, Row: WithLength, Col: WithLength> {
    current: GridIndex<Row, Col>, // no need for an Option as we're only incrementing
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for RightGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Col> + WithLength,
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

/// An iterator with leftwards direction.
/// On each step it's decrementing `col` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct LeftGridIterator<'a, T, Row: WithLength, Col: WithLength> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for LeftGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Col> + WithLength,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.is_valid() {
                let old_current = current;
                if current.col == 0.into() {
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

/// An iterator with upwards direction.
/// On each step it's decrementing `row` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct TopGridIterator<'a, T, Row: WithLength, Col: WithLength> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for TopGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Row> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + WithLength,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.is_valid() {
                let old_current = current;
                if current.row == 0.into() {
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

/// An iterator with downwards direction.
/// On each step it's incrementing `row` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct BottomGridIterator<'a, T, Row: WithLength, Col: WithLength> {
    current: GridIndex<Row, Col>, // no need for an Option as we're only incrementing
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for BottomGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Row> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + WithLength,
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

/// A diagonal iterator with top-left direction.
/// On each step it's decrementing `col` and `row` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct TopLeftGridIterator<'a, T, Row: WithLength, Col: WithLength> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for TopLeftGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Row> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Col> + WithLength,
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

/// A diagonal iterator with top-right direction.
/// On each step it's decrementing `row` and incrementing `col` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct TopRightGridIterator<'a, T, Row: WithLength, Col: WithLength> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for TopRightGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Row> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Col> + WithLength,
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

/// A diagonal iterator with bottom-right direction.
/// On each step it's incrementing `col` and `row` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct BottomRightGridIterator<'a, T, Row: WithLength, Col: WithLength> {
    current: GridIndex<Row, Col>, // no need for an Option as we're only incrementing
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for BottomRightGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Row> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Col> + WithLength,
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

/// A diagonal iterator with bottom-left direction.
/// On each step it's incrementing `row` and decrementing `col` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct BottomLeftGridIterator<'a, T, Row: WithLength, Col: WithLength> {
    current: Option<GridIndex<Row, Col>>,
    grid: &'a Grid<T, Row, Col>,
}

impl<'a, T, Row, Col> Iterator for BottomLeftGridIterator<'a, T, Row, Col>
where
    T: Default,
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize, Output = Row> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize, Output = Col> + WithLength,
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

/// Needed to create iterator adapter which gives the current iteration [`GridIndex`]
/// as well as the next value.
pub trait WithGridIndex<Row, Col> {
    /// Returns current [`GridIndex`] if it is valid, otherwise [`None`].
    fn get_index(&self) -> Option<GridIndex<Row, Col>>;

    /// Returns an iterator which gives the current iteration [`GridIndex`]
    /// as well as the next value.
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
    Row: Copy + WithLength,
    Col: Copy + WithLength,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        Some(self.current)
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for LeftGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithLength,
    Col: Copy + WithLength,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for TopGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithLength,
    Col: Copy + WithLength,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for BottomGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithLength,
    Col: Copy + WithLength,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        Some(self.current)
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for TopLeftGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithLength,
    Col: Copy + WithLength,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for TopRightGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithLength,
    Col: Copy + WithLength,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for BottomRightGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithLength,
    Col: Copy + WithLength,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        Some(self.current)
    }
}

impl<T, Row, Col> WithGridIndex<Row, Col> for BottomLeftGridIterator<'_, T, Row, Col>
where
    Row: Copy + WithLength,
    Col: Copy + WithLength,
{
    fn get_index(&self) -> Option<GridIndex<Row, Col>> {
        self.current
    }
}

/// An iterator that yields the current [`GridIndex`] and the element during iteration.
pub struct IndexedGridIterator<Row, Col, It> {
    it: It,
    phantom_data: PhantomData<(Row, Col)>,
}

impl<Row, Col, It> Iterator for IndexedGridIterator<Row, Col, It>
where
    Row: Copy + Into<usize> + PartialOrd + From<usize> + Add<usize> + WithLength,
    Col: Copy + Into<usize> + PartialOrd + From<usize> + Sub<usize> + WithLength,
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
