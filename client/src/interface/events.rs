use bevy::prelude::*;

use crate::game::GameInfo;
use crate::util;

util::entity_type!(
    /// Event that indicates that the game is ready to be played.
    /// Contains game entity.
    GameReady, Event
);

util::entity_type!(
    /// Event that indicates that all gameplay and visual processes are finished
    /// and game is ready to be closed.
    /// Contains game entity.
    GameReadyToExit, Event
);

util::entity_type!(
    /// Event that indicates that the app had left `AppState::Game` and
    /// game visuals has been despawned.
    /// Contains game entity.
    GameLeft, Event
);

util::entity_type!(
    /// Event that indicates that submit button is pressed.
    SubmitPressed, Event
);

util::entity_type!(
    /// Event that indicates that the button related to a particular setting option is pressed.
    SettingOptionPressed, Event
);

/// Event that indicates that the list of games this player can play is ready.
#[derive(Debug, Event)]
pub struct PlayerGamesReady {
    pub games: Result<Vec<GameInfo>, String>,
}

/// Event that indicates that join game button is pressed.
#[derive(Debug, Event)]
pub struct JoinPressed {
    game_id: u64,
}

impl JoinPressed {
    pub fn new(game_id: u64) -> Self {
        Self { game_id }
    }

    pub fn game_id(&self) -> u64 {
        self.game_id
    }
}
