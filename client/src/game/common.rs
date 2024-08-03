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

#[derive(Clone, Copy, Debug)]
pub enum EnemyType<T> {
    User(u64),
    Bot(T),
}
