use bevy::utils::thiserror;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum GameError {
    #[error("protobuf message received from server is invalid: {reason}")]
    InvalidProtobufMessage { reason: String },
}
