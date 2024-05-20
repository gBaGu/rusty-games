use std::iter::{Cycle, Peekable};
use std::vec::IntoIter;

pub type PlayerId = u64;

pub trait WithPlayerId {
    fn get_id(&self) -> PlayerId;
}

pub trait PlayerQueue {
    type Item: WithPlayerId;

    fn get_all(&self) -> &[Self::Item];

    fn find<F>(&self, f: F) -> Option<&Self::Item>
    where
        F: FnMut(&&Self::Item) -> bool;

    fn find_by_id(&self, id: PlayerId) -> Option<&Self::Item>;

    fn get_current(&mut self) -> Option<&Self::Item>;

    fn next(&mut self) -> Option<&Self::Item>;
}

#[derive(Debug)]
pub struct PlayerPool<T: Clone> {
    players: Vec<T>,
    players_queue: Peekable<Cycle<IntoIter<T>>>,
}

impl<T: Clone> PlayerPool<T> {
    pub fn new(players: Vec<T>) -> Self {
        Self {
            players: players.clone(),
            players_queue: players.into_iter().cycle().peekable(),
        }
    }
}

impl<T: Clone + WithPlayerId> PlayerQueue for PlayerPool<T> {
    type Item = T;

    fn get_all(&self) -> &[T] {
        self.players.as_slice()
    }

    fn find<F>(&self, f: F) -> Option<&T>
    where
        F: FnMut(&&T) -> bool,
    {
        self.players.iter().find(f)
    }

    fn find_by_id(&self, id: PlayerId) -> Option<&T> {
        self.players.iter().find(|player| player.get_id() == id)
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
        id: PlayerId,
        some_data: usize,
    }

    impl DummyPlayer {
        pub fn new(id: PlayerId, some_data: usize) -> Self {
            Self { id, some_data }
        }
    }

    impl WithPlayerId for DummyPlayer {
        fn get_id(&self) -> PlayerId {
            self.id
        }
    }

    impl<T> WithPlayerId for T
    where
        T: Clone + Copy + Into<PlayerId>
    {
        fn get_id(&self) -> PlayerId {
            (*self).into()
        }
    }

    #[test]
    fn test_find() {
        let pool = PlayerPool::new(vec![
            DummyPlayer::new(0, 12),
            DummyPlayer::new(1, 256),
            DummyPlayer::new(2, 1),
            DummyPlayer::new(3, 1),
            DummyPlayer::new(4, 0),
            DummyPlayer::new(5, 37),
        ]);

        assert_eq!(
            pool.find(|&&p| p.id == 3).cloned(),
            Some(DummyPlayer::new(3, 1))
        );
        assert_eq!(
            pool.find(|&&p| p.some_data == 1).cloned(),
            Some(DummyPlayer::new(2, 1))
        );
        assert_eq!(
            pool.find(|&&p| p.some_data == 1 && p.id == 3).cloned(),
            Some(DummyPlayer::new(3, 1))
        );
        assert_eq!(pool.find(|&&p| p.id == 6), None);
    }

    #[test]
    fn test_find_by_id() {
        let pool = PlayerPool::new(vec![
            DummyPlayer::new(3, 45),
            DummyPlayer::new(4, 9),
            DummyPlayer::new(7, 42),
            DummyPlayer::new(2, 21),
            DummyPlayer::new(9, 10),
            DummyPlayer::new(5, 5),
        ]);

        assert_eq!(pool.find_by_id(3).cloned(), Some(DummyPlayer::new(3, 45)));
        assert_eq!(pool.find_by_id(5).cloned(), Some(DummyPlayer::new(5, 5)));
        assert_eq!(pool.find_by_id(1).cloned(), None);
    }

    #[test]
    fn test_get_current() {
        let mut pool = PlayerPool::new(vec![5u64, 1, 2, 2, 3]);

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
        let mut pool = PlayerPool::new(vec![1u64, 2, 3]);
        // check that we are starting with the first element
        assert_eq!(pool.get_current(), Some(&1));
        // check that elements cycle endlessly
        itertools::assert_equal(
            std::iter::from_fn(|| pool.next().cloned()).take(10),
            [2, 3, 1, 2, 3, 1, 2, 3, 1, 2],
        );
    }

    #[test]
    fn test_get_all() {
        let mut pool = PlayerPool::new(vec![1u64, 2, 3]);

        // initial sequence is returned
        itertools::assert_equal(pool.get_all(), &[1, 2, 3]);

        // advancing the queue doesn't affect get_all
        pool.next();
        itertools::assert_equal(pool.get_all(), &[1, 2, 3]);
    }
}
