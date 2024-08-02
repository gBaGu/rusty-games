use bevy::prelude::*;

#[derive(Clone, Copy, Debug)]
pub enum BotDifficulty {
    Easy,
    Medium,
    Hard,
}

impl BotDifficulty {
    pub fn filename(&self) -> String {
        match self {
            Self::Easy => "easy".to_string(),
            Self::Medium => "medium".to_string(),
            Self::Hard => "hard".to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct BotStrategy<T>(T);

impl<T> BotStrategy<T> {
    pub fn new(strategy: T) -> Self {
        Self(strategy)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum EnemyType<T> {
    User(u64),
    Bot(BotStrategy<T>),
}
