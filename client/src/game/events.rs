use bevy::prelude::*;
use game_server::game::{GameState, PlayerId};

#[derive(Debug, Event)]
pub struct CreateGame<T> {
    id: Option<u64>,
    context: T,
}

impl<T> CreateGame<T> {
    pub fn new(id: Option<u64>, ctx: T) -> Self {
        Self { id, context: ctx }
    }

    pub fn new_over_network(id: u64, ctx: T) -> Self {
        Self::new(Some(id), ctx)
    }

    pub fn new_local(ctx: T) -> Self {
        Self::new(None, ctx)
    }

    pub fn id(&self) -> Option<u64> {
        self.id
    }

    pub fn context(&self) -> &T {
        &self.context
    }
}

#[derive(Clone, Copy, Debug, Event)]
pub struct BotReady {
    bot: Entity,
    game: Entity,
    player_position: PlayerId,
}

impl BotReady {
    pub fn new(bot: Entity, game: Entity, player_position: PlayerId) -> Self {
        Self {
            bot,
            game,
            player_position,
        }
    }

    pub fn bot(&self) -> Entity {
        self.bot
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player_position(&self) -> PlayerId {
        self.player_position
    }
}

#[derive(Clone, Copy, Debug, Event)]
pub struct PlayerActionInitialized<T> {
    game: Entity,
    player: PlayerId,
    action: T,
}

impl<T: Clone + Copy> PlayerActionInitialized<T> {
    pub fn new(game: Entity, player: PlayerId, action: T) -> Self {
        Self {
            game,
            player,
            action,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> PlayerId {
        self.player
    }

    pub fn action(&self) -> T {
        self.action
    }
}

#[derive(Clone, Copy, Debug, Event)]
pub struct PlayerActionApplied<T> {
    game: Entity,
    player: PlayerId,
    action: T,
}

impl<T: Clone + Copy> PlayerActionApplied<T> {
    pub fn new(game: Entity, player: PlayerId, action: T) -> Self {
        Self {
            game,
            player,
            action,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> PlayerId {
        self.player
    }

    pub fn action(&self) -> T {
        self.action
    }
}

#[derive(Debug, Event)]
pub struct TurnStart {
    game: Entity,
    player: PlayerId,
}

impl TurnStart {
    pub fn new(game: Entity, player: PlayerId) -> Self {
        Self {
            game,
            player,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> PlayerId {
        self.player
    }
}

#[derive(Debug, Event)]
pub struct StateUpdated {
    game: Entity,
    state: GameState,
}

impl StateUpdated {
    pub fn new(game: Entity, state: GameState) -> Self {
        Self {
            game,
            state,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn state(&self) -> GameState {
        self.state
    }
}

#[derive(Debug, Event)]
pub struct Draw {
    game: Entity,
}

impl Draw {
    pub fn new(game: Entity) -> Self {
        Self { game }
    }

    pub fn game(&self) -> Entity {
        self.game
    }
}

#[derive(Debug, Event)]
pub struct PlayerWon {
    game: Entity,
    player: PlayerId,
}

impl PlayerWon {
    pub fn new(game: Entity, player: PlayerId) -> Self {
        Self {
            game,
            player,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> PlayerId {
        self.player
    }
}
