use std::marker::PhantomData;

use bevy::prelude::*;
use game_server::core;
use smallvec::SmallVec;

use super::PendingAction;
use crate::interface::common::{PRIMARY_COLOR, SECONDARY_COLOR};
use crate::interface::GameSettingsLink;

/// Empty component to indicate that an entity is a board.
#[derive(Component)]
pub struct Board;

/// Component that specifies a difficulty of a bot.
#[derive(Clone, Copy, Debug, Component)]
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

/// Component that indicates that entity is related to a particular game.
#[derive(Debug, Component, Deref)]
pub struct GameLink(Entity);

impl GameLink {
    pub fn new(game: Entity) -> Self {
        Self(game)
    }

    pub fn get(&self) -> Entity {
        self.0
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
/// Contains [`GameLink`], [`SpriteBundle`] and a [`Board`].
#[derive(Bundle)]
pub struct BoardBundle {
    game_link: GameLink,
    background: SpriteBundle,
    board: Board,
}

impl BoardBundle {
    pub fn new(game: Entity, size: Vec2, translation: Vec3) -> Self {
        Self {
            game_link: GameLink::new(game),
            background: SpriteBundle {
                sprite: Sprite {
                    color: SECONDARY_COLOR,
                    custom_size: Some(size),
                    ..default()
                },
                transform: Transform::from_translation(translation),
                ..default()
            },
            board: Board,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct BotDifficultyButtonBundle {
    pub button: ButtonBundle,
    pub difficulty: BotDifficulty,
    pub settings_link: GameSettingsLink,
}

impl BotDifficultyButtonBundle {
    pub fn new(style: Style, difficulty: BotDifficulty, settings: Entity) -> Self {
        Self {
            button: ButtonBundle {
                style,
                background_color: PRIMARY_COLOR.into(),
                ..default()
            },
            difficulty,
            settings_link: GameSettingsLink::new(settings),
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
    A: Send + Sync + 'static
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
    A: Send + Sync + 'static
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
    A: Send + Sync + 'static
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
