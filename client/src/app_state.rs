use bevy::prelude::{Component, States};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
pub enum MenuState {
    #[default]
    Main,
    Game,
    PlayAgainstBot,
    PlayOverNetwork,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, States)]
pub enum AppState {
    Menu(MenuState),
    Game,
    Paused,
}

impl Default for AppState {
    fn default() -> Self {
        Self::Menu(MenuState::default())
    }
}

#[derive(Component, Debug)]
pub struct AppStateTransition(pub Option<AppState>);