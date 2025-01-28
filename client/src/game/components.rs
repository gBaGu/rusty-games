use std::fmt;
use std::marker::PhantomData;

use bevy::prelude::*;
use game_server::core;
use smallvec::SmallVec;

use super::{ConfirmationStatus, PendingAction, ACTION_RESEND_INTERVAL_SEC};
use crate::{interface, util};

util::entity_type!(
    /// Component that indicates that entity is related to a particular game.
    GameLink, Component
);

/// Empty component to indicate that an entity is a board.
#[derive(Component)]
pub struct Board;

/// Component that specifies a difficulty of a bot.
#[derive(Clone, Copy, Debug, PartialEq, Component)]
pub enum BotDifficulty {
    Easy,
    Medium,
    Hard,
}

impl fmt::Display for BotDifficulty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
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

/// Component that indicates that the entity is a game.
#[derive(Debug, Component)]
pub struct Game;

/// Component that indicates finished game.
#[derive(Debug, Component)]
pub struct FinishedGame;

/// Component that indicates that the game is being played now.
#[derive(Debug, Component)]
pub struct ActiveGame;

/// Component that indicates that the game is stored on the server by the id this component stores.
#[derive(Clone, Copy, Debug, Component, Deref, DerefMut)]
pub struct NetworkGame(u64);

/// Component that indicates that the game is waiting for the server reply to be created locally.
#[derive(Debug, Component)]
pub struct PendingGame<T>(PhantomData<T>);

impl<T> Default for PendingGame<T> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

/// Contains actions that are waiting to be applied.
#[derive(Debug, Component, Deref, DerefMut)]
pub struct PendingActionQueue<T>(SmallVec<[PendingAction<T>; 8]>);

impl<T> Default for PendingActionQueue<T> {
    fn default() -> Self {
        Self(SmallVec::default())
    }
}

impl<T> From<SmallVec<[PendingAction<T>; 8]>> for PendingActionQueue<T> {
    fn from(value: SmallVec<[PendingAction<T>; 8]>) -> Self {
        Self(value)
    }
}

impl<T> PendingActionQueue<T> {
    /// Remove all confirmed actions and return iterator that yields them.
    pub fn pop_confirmed(&mut self) -> impl Iterator<Item = PendingAction<T>> + '_ {
        self.0.drain_filter(|a| a.is_confirmed())
    }

    /// Confirm last consecutive unconfirmed actions and return iterator that yields them.
    pub fn confirm_latest(&mut self) -> impl Iterator<Item = &PendingAction<T>> {
        let mut confirmed_count = 0;
        for action in self.iter_mut().rev().take_while(|a| !a.is_confirmed()) {
            action.set_status(ConfirmationStatus::Confirmed);
            confirmed_count += 1;
        }
        self[self.len() - confirmed_count..].iter()
    }
}

///  Prevents actions from being sent for confirmation.  
/// Contains [`Timer`] that defines for how long actions cannot be sent for confirmation.
#[derive(Debug, Component, Deref, DerefMut)]
pub struct ActionResendTimer(Timer);

impl Default for ActionResendTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            ACTION_RESEND_INTERVAL_SEC,
            TimerMode::Once,
        ))
    }
}

/// Local game instance.
/// If [`NetworkGame`] is present this instance will reflect the one on the server side.
#[derive(Debug, Default, Component, Deref, DerefMut)]
pub struct LocalGame<T>(T);

impl<T: core::Game> From<T> for LocalGame<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

/// Component that stores position of a player in the game player queue.
#[derive(Clone, Copy, Debug, PartialEq, Component, Deref, DerefMut)]
pub struct PlayerPosition(core::PlayerPosition);

impl PlayerPosition {
    pub fn new(player: core::PlayerPosition) -> Self {
        Self(player)
    }
}

/// Component that indicates that the player is managed by a user with id this component stores.
#[derive(Clone, Copy, Debug, Component, Deref, DerefMut)]
pub struct UserAuthority(u64);

impl UserAuthority {
    pub fn new(user: u64) -> Self {
        Self(user)
    }
}

/// Component that indicates that the player is managed by a bot with id this component stores.
#[derive(Clone, Copy, Debug, Component, Deref, DerefMut)]
pub struct BotAuthority(u64);

impl BotAuthority {
    pub fn new(bot: u64) -> Self {
        Self(bot)
    }
}

/// Indicates player that is currently logged in the game.
#[derive(Debug, Component)]
pub struct CurrentUser;

/// Indicates player that is currently authorized to perform action(s) in the game.
#[derive(Debug, Component)]
pub struct CurrentPlayer;

/// Indicates player that won the game.
#[derive(Debug, Component)]
pub struct Winner;

/// Bundle for a board.
/// Contains [`GameLink`], [`Sprite`], [`Transform`] and a [`Board`].
#[derive(Bundle)]
pub struct BoardBundle {
    game_link: GameLink,
    sprite: Sprite,
    transform: Transform,
    board: Board,
}

impl BoardBundle {
    pub fn new(game: Entity, size: Vec2, translation: Vec3) -> Self {
        Self {
            game_link: game.into(),
            sprite: Sprite {
                color: interface::common::SECONDARY_COLOR,
                custom_size: Some(size),
                ..default()
            },
            transform: Transform::from_translation(translation),
            board: Board,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct PendingNewGameBundle<T: Send + Sync + 'static> {
    pending_game: PendingGame<T>,
}

impl<T: Send + Sync + 'static> PendingNewGameBundle<T> {
    pub fn new() -> Self {
        Self {
            pending_game: PendingGame::default(),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct PendingExistingGameBundle<T: Send + Sync + 'static> {
    pending_game: PendingGame<T>,
    network_game: NetworkGame,
}

impl<T: Send + Sync + 'static> PendingExistingGameBundle<T> {
    pub fn new(id: u64) -> Self {
        Self {
            pending_game: PendingGame::default(),
            network_game: NetworkGame(id),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct LocalGameBundle<G: Send + Sync + 'static, A: Send + Sync + 'static> {
    pub local_game: LocalGame<G>,
    pub pending_actions: PendingActionQueue<A>,
    game: Game,
}

impl<G, A> Default for LocalGameBundle<G, A>
where
    G: Default + Send + Sync + 'static,
    A: Send + Sync + 'static,
{
    fn default() -> Self {
        Self {
            local_game: Default::default(),
            pending_actions: Default::default(),
            game: Game,
        }
    }
}

impl<G, A> From<G> for LocalGameBundle<G, A>
where
    G: core::Game + Send + Sync + 'static,
    A: Send + Sync + 'static,
{
    fn from(value: G) -> Self {
        Self {
            local_game: LocalGame::from(value),
            pending_actions: Default::default(),
            game: Game,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct NetworkGameBundle<G: Send + Sync + 'static, A: Send + Sync + 'static> {
    pub id: NetworkGame,
    pub local_game: LocalGame<G>,
    pub pending_actions: PendingActionQueue<A>,
    game: Game,
}

impl<G, A> NetworkGameBundle<G, A>
where
    G: Send + Sync + 'static,
    A: Send + Sync + 'static,
{
    pub fn new(id: u64, game: G) -> Self {
        Self {
            id: NetworkGame(id),
            local_game: LocalGame(game),
            pending_actions: Default::default(),
            game: Game,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct CurrentUserPlayerBundle {
    player: PlayerPosition,
    auth: UserAuthority,
    current_user: CurrentUser,
}

impl CurrentUserPlayerBundle {
    pub fn new(id: u64, player_position: core::PlayerPosition) -> Self {
        Self {
            player: PlayerPosition(player_position),
            auth: UserAuthority::new(id),
            current_user: CurrentUser,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct NetworkPlayerBundle {
    player: PlayerPosition,
    auth: UserAuthority,
}

impl NetworkPlayerBundle {
    pub fn new(id: u64, player_position: core::PlayerPosition) -> Self {
        Self {
            player: PlayerPosition(player_position),
            auth: UserAuthority::new(id),
        }
    }
}

/// Component that stores a position inside the board.
#[derive(Clone, Copy, Debug, PartialEq, Component, Deref, DerefMut)]
pub struct Position(core::GridIndex);

impl Position {
    pub fn new(row: usize, col: usize) -> Self {
        Self(core::GridIndex::new(row, col))
    }
}

impl From<core::GridIndex> for Position {
    fn from(value: core::GridIndex) -> Self {
        Self(value)
    }
}

impl From<Position> for core::GridIndex {
    fn from(value: Position) -> Self {
        value.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn action_queue_pop_confirmed() {
        // remove first two and leave the rest
        let mut queue = PendingActionQueue::from(smallvec::smallvec![
            PendingAction::new(0, 0, ConfirmationStatus::Confirmed),
            PendingAction::new(0, 1, ConfirmationStatus::Confirmed),
            PendingAction::new(0, 2, ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, 3, ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, 4, ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, 5, ConfirmationStatus::NotConfirmed),
        ]);
        itertools::assert_equal(
            queue.pop_confirmed(),
            [
                PendingAction::new(0, 0, ConfirmationStatus::Confirmed),
                PendingAction::new(0, 1, ConfirmationStatus::Confirmed),
            ]
            .into_iter(),
        );
        itertools::assert_equal(
            queue.iter(),
            [
                PendingAction::new(0, 2, ConfirmationStatus::WaitingConfirmation),
                PendingAction::new(0, 3, ConfirmationStatus::WaitingConfirmation),
                PendingAction::new(0, 4, ConfirmationStatus::NotConfirmed),
                PendingAction::new(0, 5, ConfirmationStatus::NotConfirmed),
            ]
            .iter(),
        );

        // not consecutive confirmed actions will be removed as well in this implementation
        queue = PendingActionQueue::from(smallvec::smallvec![
            PendingAction::new(0, 0, ConfirmationStatus::Confirmed),
            PendingAction::new(0, 1, ConfirmationStatus::Confirmed),
            PendingAction::new(0, 2, ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, 3, ConfirmationStatus::Confirmed),
            PendingAction::new(0, 4, ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, 5, ConfirmationStatus::Confirmed),
        ]);
        itertools::assert_equal(
            queue.pop_confirmed(),
            [
                PendingAction::new(0, 0, ConfirmationStatus::Confirmed),
                PendingAction::new(0, 1, ConfirmationStatus::Confirmed),
                PendingAction::new(0, 3, ConfirmationStatus::Confirmed),
                PendingAction::new(0, 5, ConfirmationStatus::Confirmed),
            ]
            .into_iter(),
        );
        itertools::assert_equal(
            queue.iter(),
            [
                PendingAction::new(0, 2, ConfirmationStatus::WaitingConfirmation),
                PendingAction::new(0, 4, ConfirmationStatus::NotConfirmed),
            ]
            .iter(),
        );

        // if there is no confirmed actions - yield nothing and leave the queue unchanged
        queue = PendingActionQueue::from(smallvec::smallvec![
            PendingAction::new(0, 1, ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, 2, ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, 3, ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, 4, ConfirmationStatus::NotConfirmed),
        ]);
        itertools::assert_equal(queue.pop_confirmed(), std::iter::empty());
        itertools::assert_equal(
            queue.iter(),
            [
                PendingAction::new(0, 1, ConfirmationStatus::WaitingConfirmation),
                PendingAction::new(0, 2, ConfirmationStatus::WaitingConfirmation),
                PendingAction::new(0, 3, ConfirmationStatus::NotConfirmed),
                PendingAction::new(0, 4, ConfirmationStatus::NotConfirmed),
            ]
            .iter(),
        );
    }

    #[test]
    fn action_queue_confirm_latest() {
        // confirm last 4
        let mut queue = PendingActionQueue::from(smallvec::smallvec![
            PendingAction::new(0, (), ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, (), ConfirmationStatus::Confirmed),
            PendingAction::new(0, (), ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, (), ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, (), ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, (), ConfirmationStatus::NotConfirmed),
        ]);
        assert_eq!(queue.confirm_latest().count(), 4);
        itertools::assert_equal(
            queue.iter().map(|a| a.status()),
            std::iter::once(ConfirmationStatus::NotConfirmed)
                .chain(std::iter::repeat(ConfirmationStatus::Confirmed).take(5)),
        );

        // the last one is confirmed, so leave the rest unchanged
        queue = PendingActionQueue::from(smallvec::smallvec![
            PendingAction::new(0, (), ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, (), ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, (), ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, (), ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, (), ConfirmationStatus::Confirmed),
        ]);
        assert_eq!(queue.confirm_latest().count(), 0);
        itertools::assert_equal(
            queue.iter().map(|a| a.status()),
            std::iter::once(ConfirmationStatus::WaitingConfirmation)
                .chain(std::iter::repeat(ConfirmationStatus::NotConfirmed).take(3))
                .chain(std::iter::once(ConfirmationStatus::Confirmed)),
        );

        // confirm all
        queue = PendingActionQueue::from(smallvec::smallvec![
            PendingAction::new(0, (), ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, (), ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, (), ConfirmationStatus::NotConfirmed),
            PendingAction::new(0, (), ConfirmationStatus::NotConfirmed),
        ]);
        assert_eq!(queue.confirm_latest().count(), 4);
        itertools::assert_equal(
            queue.iter().map(|a| a.status()),
            std::iter::repeat(ConfirmationStatus::Confirmed).take(4),
        );
    }
}
