use bevy::utils::thiserror;
use game_server::game::encoding::FromProtobufError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum GameError {
    #[error(transparent)]
    MessageDataConversion(#[from] FromProtobufError),
}
