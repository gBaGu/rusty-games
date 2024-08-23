use std::marker::PhantomData;

use bevy::prelude::*;
use game_server::game::grid::GridIndex;
use game_server::game::{Game, PlayerId};

use crate::interface::common::{PRIMARY_COLOR, SECONDARY_COLOR};
use crate::interface::GameSettingsLink;

/// Empty component to indicate that an entity is a board.
#[derive(Component)]
pub struct Board;

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

#[derive(Debug, Component)]
pub struct PendingAction<T> {
    player: PlayerId,
    action: T,
}

impl<T: Clone + Copy> PendingAction<T> {
    pub fn new(player: PlayerId, action: T) -> Self {
        Self {
            player,
            action,
        }
    }

    pub fn action(&self) -> T {
        self.action
    }

    pub fn player(&self) -> PlayerId {
        self.player
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Component)]
pub enum PendingActionStatus {
    NotConfirmed,
    WaitingConfirmation,
    Confirmed,
}

impl PendingActionStatus {
    pub fn is_confirmed(&self) -> bool {
        *self == Self::Confirmed
    }

    pub fn is_not_confirmed(&self) -> bool {
        *self == Self::NotConfirmed
    }
}

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

#[derive(Debug, Component)]
pub struct ActiveGame;

#[derive(Clone, Copy, Debug, Component, Deref, DerefMut)]
pub struct NetworkGame(u64);

#[derive(Debug, Component)]
pub struct PendingGame<T>(PhantomData<T>);

impl<T> Default for PendingGame<T> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

#[derive(Debug, Default, Component, Deref, DerefMut)]
pub struct LocalGame<T>(T);

impl<T: Game> From<T> for LocalGame<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Component, Deref, DerefMut)]
pub struct PlayerPosition(PlayerId);

impl PlayerPosition {
    pub fn new(player: PlayerId) -> Self {
        Self(player)
    }
}

#[derive(Clone, Copy, Debug, Component, Deref, DerefMut)]
pub struct UserAuthority(u64);

impl UserAuthority {
    pub fn new(user: u64) -> Self {
        Self(user)
    }
}

#[derive(Clone, Copy, Debug, Component, Deref, DerefMut)]
pub struct BotAuthority(u64);

impl BotAuthority {
    pub fn new(bot: u64) -> Self {
        Self(bot)
    }
}

/// Indicates player that is currently logged in and playing the game
#[derive(Debug, Component)]
pub struct CurrentUser;

/// Indicates player that is currently authorized to perform action(s) in the game
#[derive(Debug, Component)]
pub struct CurrentPlayer;

#[derive(Debug, Component)]
pub struct Winner;

/// Bundle for a board.
/// Contains [`SpriteBundle`] and a [`Board`].
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
pub struct LocalGameBundle<T: Send + Sync + 'static> {
    pub local_game: LocalGame<T>,
}

impl<T: Default + Send + Sync + 'static> Default for LocalGameBundle<T> {
    fn default() -> Self {
        Self {
            local_game: LocalGame(T::default()),
        }
    }
}

impl<T: Game + Send + Sync + 'static> From<T> for LocalGameBundle<T> {
    fn from(value: T) -> Self {
        Self {
            local_game: LocalGame::from(value),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct NetworkGameBundle<T: Send + Sync + 'static> {
    pub id: NetworkGame,
    pub local_game: LocalGame<T>,
}

impl<T: Send + Sync + 'static> NetworkGameBundle<T> {
    pub fn new(id: u64, game: T) -> Self {
        Self {
            id: NetworkGame(id),
            local_game: LocalGame(game),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct PendingActionBundle<T: Send + Sync + 'static> {
    action: PendingAction<T>,
    status: PendingActionStatus,
}

impl<T: Clone + Copy + Send + Sync + 'static> PendingActionBundle<T> {
    pub fn new(player: PlayerId, action: T, status: PendingActionStatus) -> Self {
        Self {
            action: PendingAction::new(player, action),
            status,
        }
    }

    pub fn new_confirmed(player: PlayerId, action: T) -> Self {
        Self::new(player, action, PendingActionStatus::Confirmed)
    }

    pub fn new_unconfirmed(player: PlayerId, action: T) -> Self {
        Self::new(player, action, PendingActionStatus::NotConfirmed)
    }
}

#[derive(Debug, Bundle)]
pub struct CurrentUserPlayerBundle {
    player: PlayerPosition,
    auth: UserAuthority,
    current_user: CurrentUser,
}

impl CurrentUserPlayerBundle {
    pub fn new(id: u64, player_position: PlayerId) -> Self {
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
    pub fn new(id: u64, player_position: PlayerId) -> Self {
        Self {
            player: PlayerPosition(player_position),
            auth: UserAuthority::new(id),
        }
    }
}

/////////////////////

/// Component that stores a position inside the board.
#[derive(Clone, Copy, Debug, PartialEq, Component, Deref, DerefMut)]
pub struct Position(GridIndex);

impl Position {
    pub fn new(row: usize, col: usize) -> Self {
        Self(GridIndex::new(row, col))
    }
}

impl From<GridIndex> for Position {
    fn from(value: GridIndex) -> Self {
        Self(value)
    }
}

impl From<Position> for GridIndex {
    fn from(value: Position) -> Self {
        value.0
    }
}
