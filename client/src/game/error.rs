use bevy::utils::thiserror;
use game_server::game::game::FromProtobufError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum GameError {
    #[error(transparent)]
    MessageDataConversion(#[from] FromProtobufError),
}
