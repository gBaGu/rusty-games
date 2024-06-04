use std::iter::{Cycle, Peekable};
use std::marker::PhantomData;
use smallvec::{IntoIter, SmallVec};

pub trait Player {
    type Id;

    fn id(&self) -> Self::Id;
}

pub trait PlayerQueue {
    type Id: PartialEq;
    type Item: Player<Id = Self::Id>;

    fn as_slice(&self) -> &[Self::Item];

    fn get_current(&mut self) -> Option<&Self::Item>;

    fn next(&mut self) -> Option<&Self::Item>;

    fn find(&self, id: Self::Id) -> Option<&Self::Item> {
        self.as_slice().iter().find(|player| player.id() == id)
    }

    fn find_if<F>(&self, f: F) -> Option<&Self::Item>
    where
        F: FnMut(&&Self::Item) -> bool,
    {
        self.as_slice().iter().find(f)
    }
}

/// Queue that stores only player ids
#[derive(Debug)]
pub struct PlayerIdQueue<T: Clone> {
    players: SmallVec<[T; 8]>,
    players_queue: Peekable<Cycle<IntoIter<[T; 8]>>>,
}

impl<T: Clone> PlayerIdQueue<T> {
    pub fn new(players: Vec<T>) -> Self {
        let players = SmallVec::from_vec(players);
        Self {
            players: players.clone(),
            players_queue: players.into_iter().cycle().peekable(),
        }
    }
}

impl<T: Clone + Player<Id = T> + PartialEq> PlayerQueue for PlayerIdQueue<T> {
    type Id = T;
    type Item = T;

    fn as_slice(&self) -> &[Self::Item] {
        self.players.as_slice()
    }

    fn get_current(&mut self) -> Option<&Self::Item> {
        self.players_queue.peek()
    }

    fn next(&mut self) -> Option<&Self::Item> {
        self.players_queue.next()?;
        self.players_queue.peek()
    }
}

#[derive(Debug)]
pub struct PlayerDataQueue<T: Clone, ID> {
    players: SmallVec<[T; 8]>,
    players_queue: Peekable<Cycle<IntoIter<[T; 8]>>>,
    _phantom_data: PhantomData<ID>,
}

impl<T: Clone, ID> PlayerDataQueue<T, ID> {
    pub fn new(players: Vec<T>) -> Self {
        let players = SmallVec::from_vec(players);
        Self {
            players: players.clone(),
            players_queue: players.into_iter().cycle().peekable(),
            _phantom_data: Default::default(),
        }
    }
}

impl<T: Clone + Player<Id = ID>, ID: PartialEq> PlayerQueue for PlayerDataQueue<T, ID> {
    type Id = ID;
    type Item = T;

    fn as_slice(&self) -> &[T] {
        self.players.as_slice()
    }

    /// Get next element from pool without advancing iterator
    /// &mut self is needed because Peekable can call next() on the underlying iterator
    fn get_current(&mut self) -> Option<&T> {
        self.players_queue.peek()
    }

    /// Advance iterator by one and return the next element from the pool
    fn next(&mut self) -> Option<&T> {
        self.players_queue.next()?;
        self.players_queue.peek()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct DummyPlayer {
        id: u32,
        some_data: usize,
    }

    impl DummyPlayer {
        pub fn new(id: u32, some_data: usize) -> Self {
            Self { id, some_data }
        }
    }

    impl Player for DummyPlayer {
        type Id = u32;

        fn id(&self) -> Self::Id {
            self.id
        }
    }

    impl Player for u64 {
        type Id = u64;

        fn id(&self) -> Self::Id {
            *self
        }
    }

    #[test]
    fn test_find_if() {
        let pool = PlayerDataQueue::new(vec![
            DummyPlayer::new(0, 12),
            DummyPlayer::new(1, 256),
            DummyPlayer::new(2, 1),
            DummyPlayer::new(3, 1),
            DummyPlayer::new(4, 0),
            DummyPlayer::new(5, 37),
        ]);

        assert_eq!(
            pool.find_if(|&&p| p.id == 3).cloned(),
            Some(DummyPlayer::new(3, 1))
        );
        assert_eq!(
            pool.find_if(|&&p| p.some_data == 1).cloned(),
            Some(DummyPlayer::new(2, 1))
        );
        assert_eq!(
            pool.find_if(|&&p| p.some_data == 1 && p.id == 3).cloned(),
            Some(DummyPlayer::new(3, 1))
        );
        assert_eq!(pool.find_if(|&&p| p.id == 6), None);
    }

    #[test]
    fn test_find_by_id() {
        let pool = PlayerDataQueue::new(vec![
            DummyPlayer::new(3, 45),
            DummyPlayer::new(4, 9),
            DummyPlayer::new(7, 42),
            DummyPlayer::new(2, 21),
            DummyPlayer::new(9, 10),
            DummyPlayer::new(5, 5),
        ]);

        assert_eq!(pool.find(3).cloned(), Some(DummyPlayer::new(3, 45)));
        assert_eq!(pool.find(5).cloned(), Some(DummyPlayer::new(5, 5)));
        assert_eq!(pool.find(1).cloned(), None);
    }

    #[test]
    fn test_get_current() {
        let mut pool = PlayerDataQueue::new(vec![5u64, 1, 2, 2, 3]);

        // starting with the first element
        assert_eq!(*pool.get_current().unwrap(), 5);
        // calling multiple times doesn't change anything
        assert_eq!(*pool.get_current().unwrap(), 5);

        // skip one
        let _ = pool.next().unwrap();

        // now getting the second element
        assert_eq!(*pool.get_current().unwrap(), 1);

        // skip 3
        let _ = pool.next().unwrap();
        let _ = pool.next().unwrap();
        let _ = pool.next().unwrap();

        // now getting the 5th element
        assert_eq!(*pool.get_current().unwrap(), 3);
    }

    #[test]
    fn test_cyclic_iteration() {
        let mut pool = PlayerDataQueue::new(vec![1u64, 2, 3]);
        // check that we are starting with the first element
        assert_eq!(pool.get_current(), Some(&1));
        // check that elements cycle endlessly
        itertools::assert_equal(
            std::iter::from_fn(|| pool.next().cloned()).take(10),
            [2, 3, 1, 2, 3, 1, 2, 3, 1, 2],
        );
    }

    #[test]
    fn test_as_slice() {
        let mut pool = PlayerDataQueue::new(vec![1u64, 2, 3]);

        // initial sequence is returned
        itertools::assert_equal(pool.as_slice(), &[1, 2, 3]);

        // advancing the queue doesn't affect as_slice
        pool.next();
        itertools::assert_equal(pool.as_slice(), &[1, 2, 3]);
    }
}
