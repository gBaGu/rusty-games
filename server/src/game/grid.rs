use generic_array::{ArrayLength, GenericArray};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, Index, IndexMut};

/// Index struct to access elements in the [`Grid`].
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
pub struct GridIndex {
    row: usize,
    col: usize,
}

impl From<(usize, usize)> for GridIndex {
    fn from(value: (usize, usize)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl Display for GridIndex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.col, self.row)
    }
}

impl GridIndex {
    /// Constructs a new [`GridIndex`].
    pub fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }

    /// Returns value of `self.col`
    pub fn col(&self) -> usize {
        self.col
    }

    /// Returns value of `self.row`
    pub fn row(&self) -> usize {
        self.row
    }

    /// Compares `self.row` to `other.row` and `self.col` to `other.col`.
    /// Returns `true` if the difference in both cases is less than or equal to 1.
    pub fn is_adjacent(&self, other: &GridIndex) -> bool {
        let vertically_adjacent = match self.row.partial_cmp(&other.row) {
            Some(Ordering::Equal) => true,
            Some(Ordering::Less) => self.row == other.row - 1,
            Some(Ordering::Greater) => other.row == self.row - 1,
            None => false,
        };
        let horizontally_adjacent = match self.col.partial_cmp(&other.col) {
            Some(Ordering::Equal) => true,
            Some(Ordering::Less) => self.col == other.col - 1,
            Some(Ordering::Greater) => other.col == self.col - 1,
            None => false,
        };
        vertically_adjacent && horizontally_adjacent
    }

    /// Returns new instance of [`GridIndex`] with the `col` value increased by `n`.
    pub fn move_right(&self, n: usize) -> Self {
        Self::new(self.row, self.col + n)
    }

    /// Returns new instance of [`GridIndex`] with the `col` value decreased by `n`.
    pub fn move_left(&self, n: usize) -> Self {
        Self::new(self.row, self.col - n)
    }

    /// Returns new instance of [`GridIndex`] with the `row` value decreased by `n`.
    pub fn move_up(&self, n: usize) -> Self {
        Self::new(self.row - n, self.col)
    }

    /// Returns new instance of [`GridIndex`] with the `row` value increased by `n`.
    pub fn move_down(&self, n: usize) -> Self {
        Self::new(self.row + n, self.col)
    }
}

/// Two-dimensional fixed-length array that stores values and allows to mutate them.
/// Length of array is defined by generic parameters `R` and `C`.
#[derive(Clone, Debug)]
pub struct Grid<T, R: ArrayLength, C: ArrayLength> {
    contents: GenericArray<GenericArray<T, C>, R>,
}

impl<T: Default, R: ArrayLength, C: ArrayLength> Default for Grid<T, R, C> {
    fn default() -> Self {
        Self {
            contents: Default::default(),
        }
    }
}

impl<T, R: ArrayLength, C: ArrayLength> Deref for Grid<T, R, C> {
    type Target = [GenericArray<T, C>];

    fn deref(&self) -> &Self::Target {
        self.contents.as_slice()
    }
}

impl<T: Display, R: ArrayLength, C: ArrayLength> Display for Grid<T, R, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("[\n")?;
        for row in self.deref() {
            f.write_str("[")?;
            for val in row {
                write!(f, "{}", val)?;
            }
            f.write_str("]\n")?;
        }
        f.write_str("]")
    }
}

impl<T, R: ArrayLength, C: ArrayLength> Index<GridIndex> for Grid<T, R, C> {
    type Output = T;

    fn index(&self, index: GridIndex) -> &Self::Output {
        &self.contents[index.row()][index.col()]
    }
}

impl<T, R: ArrayLength, C: ArrayLength> IndexMut<GridIndex> for Grid<T, R, C> {
    fn index_mut(&mut self, index: GridIndex) -> &mut Self::Output {
        &mut self.contents[index.row()][index.col()]
    }
}

impl<T, R: ArrayLength, C: ArrayLength> Grid<T, R, C> {
    /// Returns an iterator to indexed grid elements row by row
    pub fn all_indexed(&self) -> impl Iterator<Item = (GridIndex, &T)> {
        (0..self.contents.len())
            .map(|i| self.right_iter((i, 0).into()).indexed())
            .flatten()
    }

    /// Returns an iterator with rightwards direction that starts with a `pos`.
    pub fn right_iter(&self, pos: GridIndex) -> RightGridIterator<T, R, C> {
        RightGridIterator {
            current: pos,
            grid: &self,
        }
    }

    /// Returns an iterator with leftwards direction that starts with a `pos`.
    pub fn left_iter(&self, pos: GridIndex) -> LeftGridIterator<T, R, C> {
        LeftGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    /// Returns an iterator with upwards direction that starts with a `pos`.
    pub fn top_iter(&self, pos: GridIndex) -> TopGridIterator<T, R, C> {
        TopGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    /// Returns an iterator with downwards direction that starts with a `pos`.
    pub fn bottom_iter(&self, pos: GridIndex) -> BottomGridIterator<T, R, C> {
        BottomGridIterator {
            current: pos,
            grid: &self,
        }
    }

    /// Returns a diagonal iterator with top-left direction that starts with a `pos`.
    pub fn top_left_iter(&self, pos: GridIndex) -> TopLeftGridIterator<T, R, C> {
        TopLeftGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    /// Returns a diagonal iterator with top-right direction that starts with a `pos`.
    pub fn top_right_iter(&self, pos: GridIndex) -> TopRightGridIterator<T, R, C> {
        TopRightGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }

    /// Returns a diagonal iterator with bottom-right direction that starts with a `pos`.
    pub fn bottom_right_iter(&self, pos: GridIndex) -> BottomRightGridIterator<T, R, C> {
        BottomRightGridIterator {
            current: pos,
            grid: &self,
        }
    }

    /// Returns a diagonal iterator with bottom-left direction that starts with a `pos`.
    pub fn bottom_left_iter(&self, pos: GridIndex) -> BottomLeftGridIterator<T, R, C> {
        BottomLeftGridIterator {
            current: Some(pos),
            grid: &self,
        }
    }
}

/// An iterator with rightwards direction.
/// On each step it's incrementing `col` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct RightGridIterator<'a, T, R: ArrayLength, C: ArrayLength> {
    current: GridIndex, // no need for an Option as we're only incrementing
    grid: &'a Grid<T, R, C>,
}

impl<'a, T, R: ArrayLength, C: ArrayLength> Iterator for RightGridIterator<'a, T, R, C> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.row < R::to_usize() && self.current.col < C::to_usize() {
            let old_current = self.current;
            self.current = GridIndex::new(self.current.row, self.current.col + 1);
            return Some(&self.grid[old_current]);
        }
        None
    }
}

/// An iterator with leftwards direction.
/// On each step it's decrementing `col` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct LeftGridIterator<'a, T, R: ArrayLength, C: ArrayLength> {
    current: Option<GridIndex>,
    grid: &'a Grid<T, R, C>,
}

impl<'a, T, R: ArrayLength, C: ArrayLength> Iterator for LeftGridIterator<'a, T, R, C> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.row < R::to_usize() && current.col < C::to_usize() {
                let old_current = current;
                if current.col == 0 {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row, current.col - 1));
                }
                return Some(&self.grid[old_current]);
            }
        }
        None
    }
}

/// An iterator with upwards direction.
/// On each step it's decrementing `row` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct TopGridIterator<'a, T, R: ArrayLength, C: ArrayLength> {
    current: Option<GridIndex>,
    grid: &'a Grid<T, R, C>,
}

impl<'a, T, R: ArrayLength, C: ArrayLength> Iterator for TopGridIterator<'a, T, R, C> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.row < R::to_usize() && current.col < C::to_usize() {
                let old_current = current;
                if current.row == 0 {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row - 1, current.col));
                }
                return Some(&self.grid[old_current]);
            }
        }
        None
    }
}

/// An iterator with downwards direction.
/// On each step it's incrementing `row` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct BottomGridIterator<'a, T, R: ArrayLength, C: ArrayLength> {
    current: GridIndex, // no need for an Option as we're only incrementing
    grid: &'a Grid<T, R, C>,
}

impl<'a, T, R: ArrayLength, C: ArrayLength> Iterator for BottomGridIterator<'a, T, R, C> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.row < R::to_usize() && self.current.col < C::to_usize() {
            let old_current = self.current;
            self.current = GridIndex::new(self.current.row + 1, self.current.col);
            return Some(&self.grid[old_current]);
        }
        None
    }
}

/// A diagonal iterator with top-left direction.
/// On each step it's decrementing `col` and `row` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct TopLeftGridIterator<'a, T, R: ArrayLength, C: ArrayLength> {
    current: Option<GridIndex>,
    grid: &'a Grid<T, R, C>,
}

impl<'a, T, R: ArrayLength, C: ArrayLength> Iterator for TopLeftGridIterator<'a, T, R, C> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.row < R::to_usize() && current.col < C::to_usize() {
                let old_current = current;
                if current.row == 0 || current.col == 0 {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row - 1, current.col - 1));
                }
                return Some(&self.grid[old_current]);
            }
        }
        None
    }
}

/// A diagonal iterator with top-right direction.
/// On each step it's decrementing `row` and incrementing `col` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct TopRightGridIterator<'a, T, R: ArrayLength, C: ArrayLength> {
    current: Option<GridIndex>,
    grid: &'a Grid<T, R, C>,
}

impl<'a, T, R: ArrayLength, C: ArrayLength> Iterator for TopRightGridIterator<'a, T, R, C> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.row < R::to_usize() && current.col < C::to_usize() {
                let old_current = current;
                if current.row == 0 {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row - 1, current.col + 1));
                }
                return Some(&self.grid[old_current]);
            }
        }
        None
    }
}

/// A diagonal iterator with bottom-right direction.
/// On each step it's incrementing `col` and `row` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct BottomRightGridIterator<'a, T, R: ArrayLength, C: ArrayLength> {
    current: GridIndex, // no need for an Option as we're only incrementing
    grid: &'a Grid<T, R, C>,
}

impl<'a, T, R: ArrayLength, C: ArrayLength> Iterator for BottomRightGridIterator<'a, T, R, C> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.row < R::to_usize() && self.current.col < C::to_usize() {
            let old_current = self.current;
            self.current = GridIndex::new(self.current.row + 1, self.current.col + 1);
            return Some(&self.grid[old_current]);
        }
        None
    }
}

/// A diagonal iterator with bottom-left direction.
/// On each step it's incrementing `row` and decrementing `col` by 1 in the underlying [`GridIndex`].
/// Stops when underlying [`GridIndex`] goes out of [`Grid`] scope.
pub struct BottomLeftGridIterator<'a, T, R: ArrayLength, C: ArrayLength> {
    current: Option<GridIndex>,
    grid: &'a Grid<T, R, C>,
}

impl<'a, T, R: ArrayLength, C: ArrayLength> Iterator for BottomLeftGridIterator<'a, T, R, C> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.current {
            if current.row < R::to_usize() && current.col < C::to_usize() {
                let old_current = current;
                if current.col == 0 {
                    self.current = None;
                } else {
                    self.current = Some(GridIndex::new(current.row + 1, current.col - 1));
                }
                return Some(&self.grid[old_current]);
            }
        }
        None
    }
}

/// Needed to create iterator adapter which gives the current iteration [`GridIndex`]
/// as well as the next value.
pub trait WithGridIndex {
    /// Returns current [`GridIndex`] if it is valid, otherwise [`None`].
    fn get_index(&self) -> Option<GridIndex>;

    /// Returns an iterator which gives the current iteration [`GridIndex`]
    /// as well as the next value.
    fn indexed(self) -> IndexedGridIterator<Self>
    where
        Self: Sized,
    {
        IndexedGridIterator { it: self }
    }
}

impl<T, R: ArrayLength, C: ArrayLength> WithGridIndex for RightGridIterator<'_, T, R, C> {
    fn get_index(&self) -> Option<GridIndex> {
        Some(self.current)
    }
}

impl<T, R: ArrayLength, C: ArrayLength> WithGridIndex for LeftGridIterator<'_, T, R, C> {
    fn get_index(&self) -> Option<GridIndex> {
        self.current
    }
}

impl<T, R: ArrayLength, C: ArrayLength> WithGridIndex for TopGridIterator<'_, T, R, C> {
    fn get_index(&self) -> Option<GridIndex> {
        self.current
    }
}

impl<T, R: ArrayLength, C: ArrayLength> WithGridIndex for BottomGridIterator<'_, T, R, C> {
    fn get_index(&self) -> Option<GridIndex> {
        Some(self.current)
    }
}

impl<T, R: ArrayLength, C: ArrayLength> WithGridIndex for TopLeftGridIterator<'_, T, R, C> {
    fn get_index(&self) -> Option<GridIndex> {
        self.current
    }
}

impl<T, R: ArrayLength, C: ArrayLength> WithGridIndex for TopRightGridIterator<'_, T, R, C> {
    fn get_index(&self) -> Option<GridIndex> {
        self.current
    }
}

impl<T, R: ArrayLength, C: ArrayLength> WithGridIndex for BottomRightGridIterator<'_, T, R, C> {
    fn get_index(&self) -> Option<GridIndex> {
        Some(self.current)
    }
}

impl<T, R: ArrayLength, C: ArrayLength> WithGridIndex for BottomLeftGridIterator<'_, T, R, C> {
    fn get_index(&self) -> Option<GridIndex> {
        self.current
    }
}

/// An iterator that yields the current [`GridIndex`] and the element during iteration.
pub struct IndexedGridIterator<It> {
    it: It,
}

impl<It> Iterator for IndexedGridIterator<It>
where
    It: Iterator + WithGridIndex,
{
    type Item = (GridIndex, It::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.it.get_index();
        match self.it.next() {
            // unwrap() here is ok if next() returned Some()
            Some(item) => Some((index.unwrap(), item)),
            None => None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use generic_array::typenum;

    #[test]
    fn test_is_adjacent() {
        let ones = GridIndex::new(1, 1);
        // the same index is adjacent
        assert!(ones.is_adjacent(&GridIndex::new(1, 1)));

        // check all adjacent indices
        assert!(ones.is_adjacent(&GridIndex::new(2, 1)));
        assert!(ones.is_adjacent(&GridIndex::new(2, 2)));
        assert!(ones.is_adjacent(&GridIndex::new(1, 2)));
        assert!(ones.is_adjacent(&GridIndex::new(0, 2)));
        assert!(ones.is_adjacent(&GridIndex::new(0, 1)));
        assert!(ones.is_adjacent(&GridIndex::new(0, 0)));
        assert!(ones.is_adjacent(&GridIndex::new(1, 0)));
        assert!(ones.is_adjacent(&GridIndex::new(2, 0)));

        // check not adjacent
        assert!(!ones.is_adjacent(&GridIndex::new(0, 3)));
        assert!(!ones.is_adjacent(&GridIndex::new(1, 3)));
        assert!(!ones.is_adjacent(&GridIndex::new(2, 3)));
        assert!(!ones.is_adjacent(&GridIndex::new(3, 0)));
    }

    #[test]
    fn test_all_indexed() {
        let mut grid = Grid::<usize, typenum::U2, typenum::U2>::default();
        grid[(1, 1).into()] = 1;
        itertools::assert_equal(
            grid.all_indexed(),
            [
                ((0, 0).into(), &0),
                ((0, 1).into(), &0),
                ((1, 0).into(), &0),
                ((1, 1).into(), &1),
            ]
            .into_iter(),
        );
    }
}
