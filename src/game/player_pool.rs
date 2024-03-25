use std::iter::{Cycle, Peekable};
use std::vec::IntoIter;

pub type PlayerId = u64;

pub trait WithPlayerId {
    fn get_id(&self) -> PlayerId;
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

    pub fn find<F>(&self, f: F) -> Option<&T>
    where
        F: FnMut(&&T) -> bool,
    {
        self.players.iter().find(f)
    }

    pub fn find_by_id(&self, id: PlayerId) -> Option<&T>
    where
        T: WithPlayerId,
    {
        self.players.iter().find(|player| player.get_id() == id)
    }

    // Get next element from pool without advancing iterator
    // &mut self is needed because Peekable can call next() on the underlying iterator
    pub fn get_current(&mut self) -> Option<&T> {
        self.players_queue.peek()
    }

    // Get next element from pool and advance iterator by one
    pub fn next(&mut self) -> Option<T> {
        self.players_queue.next()
    }
}

#[cfg(test)]
mod test {
    use super::{PlayerId, PlayerPool, WithPlayerId};

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
        let mut pool = PlayerPool::new(vec![5, 1, 2, 2, 3]);

        // starting with the first element
        assert_eq!(*pool.get_current().unwrap(), 5);
        // calling multiple times doesn't change anything
        assert_eq!(*pool.get_current().unwrap(), 5);

        // advance by one
        let _ = pool.next().unwrap();

        // now getting the second element
        assert_eq!(*pool.get_current().unwrap(), 1);

        let _ = pool.next().unwrap();
        let _ = pool.next().unwrap();
        let _ = pool.next().unwrap();

        // now getting the 5th element
        assert_eq!(*pool.get_current().unwrap(), 3);
    }

    #[test]
    fn test_cyclic_iteration() {
        let mut pool = PlayerPool::new(vec![1, 2, 3]);
        // check that elements cycle endlessly
        let sequence: Vec<_> = std::iter::from_fn(|| pool.next()).take(10).collect();
        assert_eq!(sequence, vec![1, 2, 3, 1, 2, 3, 1, 2, 3, 1]);
    }
}
