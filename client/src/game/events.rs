use bevy::prelude::*;
use game_server::core;

/// Event that signals that all data required to create a game entity is ready.
#[derive(Debug, Event)]
pub struct GameDataReady {
    id: Option<u64>,
    current_user: u64,
    context: Entity,
}

impl GameDataReady {
    pub fn new(id: Option<u64>, current_user: u64, ctx: Entity) -> Self {
        Self {
            id,
            current_user,
            context: ctx,
        }
    }

    pub fn new_over_network(id: u64, current_user: u64, ctx: Entity) -> Self {
        Self::new(Some(id), current_user, ctx)
    }

    pub fn new_local(current_user: u64, ctx: Entity) -> Self {
        Self::new(None, current_user, ctx)
    }

    pub fn id(&self) -> Option<u64> {
        self.id
    }

    pub fn context(&self) -> Entity {
        self.context
    }

    pub fn current_user(&self) -> u64 {
        self.current_user
    }
}

/// Event that signals that game [`Entity`] is ready for interaction.
/// Triggered during game creation process when the game entity is spawned or
/// found within existing game entities.
#[derive(Debug, Deref, Event)]
pub struct GameEntityReady(Entity);

impl GameEntityReady {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

/// Event that signals that particular bot is ready to make some action in a game.
#[derive(Debug, Event)]
pub struct BotReady {
    bot: Entity,
    game: Entity,
    player_position: core::PlayerPosition,
}

impl BotReady {
    pub fn new(bot: Entity, game: Entity, player_position: core::PlayerPosition) -> Self {
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

    pub fn player_position(&self) -> core::PlayerPosition {
        self.player_position
    }
}

/// Event that signals that the first pending action in the queue has changed.
#[derive(Debug, Deref, Event)]
pub struct ActionQueueNextChanged(Entity);

impl ActionQueueNextChanged {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

/// Event that signals that `player` wants to make game action.
#[derive(Debug, Event)]
pub struct ActionInitialized<T> {
    game: Entity,
    player: core::PlayerPosition,
    action: T,
}

impl<T> ActionInitialized<T> {
    pub fn new(game: Entity, player: core::PlayerPosition, action: T) -> Self {
        Self {
            game,
            player,
            action,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }

    pub fn action(&self) -> &T {
        &self.action
    }
}

/// Event that signals that new pending action was added to the queue.
#[derive(Debug, Event)]
pub struct ActionEnqueued<T> {
    game: Entity,
    player: core::PlayerPosition,
    action: T,
}

impl<T> ActionEnqueued<T> {
    pub fn new(game: Entity, player: core::PlayerPosition, action: T) -> Self {
        Self {
            game,
            player,
            action,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }

    pub fn action(&self) -> &T {
        &self.action
    }
}

/// Defines what actions should have their status reverted.
#[derive(Debug, PartialEq)]
pub enum ActionStatusRevertPolicy<T> {
    All,
    Single(T),
}

/// Event that indicates that an action(s) cannot be confirmed by a server.
/// Contains game [`Entity`] and [`ActionStatusRevertPolicy`].
#[derive(Debug, Event)]
pub struct ActionConfirmationFailed<T> {
    game: Entity,
    revert_policy: ActionStatusRevertPolicy<T>,
}

impl<T> ActionConfirmationFailed<T> {
    fn new(game: Entity, revert_policy: ActionStatusRevertPolicy<T>) -> Self {
        Self {
            game,
            revert_policy,
        }
    }

    pub fn revert_all(game: Entity) -> Self {
        Self::new(game, ActionStatusRevertPolicy::All)
    }

    pub fn revert_single(game: Entity, action: T) -> Self {
        Self::new(game, ActionStatusRevertPolicy::Single(action))
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn revert_policy(&self) -> &ActionStatusRevertPolicy<T> {
        &self.revert_policy
    }
}

/// Event that signals that pending action was confirmed.
#[derive(Debug, Event)]
pub struct ActionConfirmed<T> {
    game: Entity,
    player: core::PlayerPosition,
    action: T,
}

impl<T> ActionConfirmed<T> {
    pub fn new(game: Entity, player: core::PlayerPosition, action: T) -> Self {
        Self {
            game,
            player,
            action,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }

    pub fn action(&self) -> &T {
        &self.action
    }
}

/// Event that signals that pending action had failed to execute and was removed from the queue.
#[derive(Debug, Event)]
pub struct ActionDropped<T> {
    game: Entity,
    player: core::PlayerPosition,
    action: T,
    reason: String,
}

impl<T> ActionDropped<T> {
    pub fn new(
        game: Entity,
        player: core::PlayerPosition,
        action: T,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            game,
            player,
            action,
            reason: reason.into(),
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }

    pub fn action(&self) -> &T {
        &self.action
    }

    pub fn reason(&self) -> &String {
        &self.reason
    }
}

/// Event that signals that `action` created by `player` is applied.
#[derive(Debug, Event)]
pub struct ActionApplied<T> {
    game: Entity,
    player: core::PlayerPosition,
    action: T,
}

impl<T> ActionApplied<T> {
    pub fn new(game: Entity, player: core::PlayerPosition, action: T) -> Self {
        Self {
            game,
            player,
            action,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }

    pub fn action(&self) -> &T {
        &self.action
    }
}

/// Event that signals that `player` is now authorized to make actions in tha game.
#[derive(Debug, Event)]
pub struct TurnStart {
    game: Entity,
    player: core::PlayerPosition,
}

impl TurnStart {
    pub fn new(game: Entity, player: core::PlayerPosition) -> Self {
        Self { game, player }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }
}

/// Event that signals that the game state is updated.
#[derive(Debug, Event)]
pub struct StateUpdated {
    game: Entity,
    state: core::GameState,
}

impl StateUpdated {
    pub fn new(game: Entity, state: core::GameState) -> Self {
        Self { game, state }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn state(&self) -> core::GameState {
        self.state
    }
}

/// Event that signals that the game is finished with a draw.
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

/// Event that signals that the game is finished with a win of `player`.
#[derive(Debug, Event)]
pub struct PlayerWon {
    game: Entity,
    player: core::PlayerPosition,
}

impl PlayerWon {
    pub fn new(game: Entity, player: core::PlayerPosition) -> Self {
        Self { game, player }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }
}
