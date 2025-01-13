use bevy::prelude::*;

use crate::game::BotDifficulty;
use crate::game::tic_tac_toe::bot::Strategy;
use crate::interface::common::PRIMARY_COLOR;
use crate::interface::GameSettingsLink;

/// Component that stores settings that are common for both bot and network games.
#[derive(Debug, Component)]
pub struct CommonGameSettings {
    user_first: bool,
}

impl Default for CommonGameSettings {
    fn default() -> Self {
        Self {
            user_first: true,
        }
    }
}

impl CommonGameSettings {
    pub fn user_first(&self) -> bool {
        self.user_first
    }
}

/// Component that stores settings related only to a network game.
#[derive(Debug, Default, Component)]
pub struct NetworkGameSettings {
    opponent: Option<u64>,
}

impl NetworkGameSettings {
    pub fn opponent(&self) -> Option<u64> {
        self.opponent
    }

    pub fn set_opponent(&mut self, id: u64) {
        let _ = self.opponent.insert(id);
    }
}

/// Component that stores settings related only to a game with bot.
#[derive(Debug, Default, Component)]
pub struct BotGameSettings {
    strategy: Option<Strategy>,
    difficulty: Option<BotDifficulty>,
}

impl BotGameSettings {
    pub fn strategy(&self) -> Option<Strategy> {
        self.strategy
    }

    pub fn difficulty(&self) -> Option<BotDifficulty> {
        self.difficulty
    }

    pub fn set_strategy(&mut self, strategy: Strategy) {
        let _ = self.strategy.insert(strategy);
    }

    pub fn set_difficulty(&mut self, difficulty: BotDifficulty) {
        let _ = self.difficulty.insert(difficulty);
    }
}

#[derive(Debug, Default, Bundle)]
pub struct BotGameSettingsBundle {
    pub common: CommonGameSettings,
    pub bot_settings: BotGameSettings,
}

#[derive(Debug, Default, Bundle)]
pub struct NetworkGameSettingsBundle {
    pub common: CommonGameSettings,
    pub bot_settings: NetworkGameSettings,
}

#[derive(Debug, Bundle)]
pub struct BotStrategyButtonBundle {
    node: Node,
    background_color: BackgroundColor,
    button: Button,
    strategy: Strategy,
    settings_link: GameSettingsLink,
}

impl BotStrategyButtonBundle {
    pub fn new(node: Node, strategy: Strategy, settings: Entity) -> Self {
        Self {
            node,
            background_color: PRIMARY_COLOR.into(),
            button: Button,
            strategy,
            settings_link: GameSettingsLink::new(settings),
        }
    }
}
