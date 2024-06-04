use bevy::utils::thiserror;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum GameError {
    #[error("user {user} does not belong to game {game}")]
    ForeignGame {
        user: u64,
        game: u64,
    }
}