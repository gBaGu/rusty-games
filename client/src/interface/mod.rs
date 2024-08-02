pub mod common;

mod components;
mod events;
mod game_list;
mod resources;
mod systems;

use bevy::prelude::*;
use bevy_simple_text_input::TextInputPlugin;

pub use components::{PlayerColor, Playground};
pub use events::{GameReady, GameReadyToExit};

use crate::app_state::{AppState, MenuState};
use crate::grpc::NetworkSystems;
use events::{PlayerGamesReady, SubmitPressed};
use resources::RefreshGamesTimer;
use systems::*;
use crate::interface::events::GameExit;

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextInputPlugin)
            .init_resource::<RefreshGamesTimer>()
            .add_event::<GameReady>()
            .add_event::<GameReadyToExit>()
            .add_event::<GameExit>()
            .add_event::<SubmitPressed>()
            .add_event::<PlayerGamesReady>()
            .add_systems(OnEnter(AppState::Menu(MenuState::Main)), setup_main_menu)
            .add_systems(OnExit(AppState::Menu(MenuState::Main)), cleanup_ui)
            .add_systems(
                OnEnter(AppState::Menu(MenuState::Settings)),
                setup_settings_menu,
            )
            .add_systems(OnExit(AppState::Menu(MenuState::Settings)), cleanup_ui)
            .add_systems(
                OnEnter(AppState::Menu(MenuState::PlayWithBot)),
                setup_play_with_bot_menu,
            )
            .add_systems(OnExit(AppState::Menu(MenuState::PlayWithBot)), cleanup_ui)
            .add_systems(
                OnEnter(AppState::Menu(MenuState::PlayOverNetwork)),
                setup_play_over_network_menu,
            )
            .add_systems(
                OnExit(AppState::Menu(MenuState::PlayOverNetwork)),
                cleanup_ui,
            )
            .add_systems(OnEnter(AppState::Paused), setup_pause)
            .add_systems(
                OnExit(AppState::Paused),
                (
                    clear_pause_overlay,
                    exit_game.run_if(not(in_state(AppState::Game))),
                ),
            )
            .add_systems(
                OnEnter(AppState::Game),
                setup_game.run_if(not(any_with_component::<Playground>)),
            )
            .add_systems(
                OnExit(AppState::Game),
                (cleanup_ui, exit_game.before(cleanup_ui)).run_if(not(in_state(AppState::Paused))),
            )
            .add_systems(
                Update,
                (
                    game_list::update,
                    game_list::on_connect,
                    game_list::on_disconnect,
                    start_game,
                    toggle_pause,
                    state_transition,
                    submit_press,
                    text_input_focus,
                    submit_setting.run_if(in_state(AppState::Menu(MenuState::Settings))),
                    (
                        (send_get_player_games, create_game, join_game).in_set(NetworkSystems),
                        handle_get_player_games,
                        handle_player_games,
                    )
                        .run_if(in_state(AppState::Menu(MenuState::PlayOverNetwork))),
                    create_game_over_overlay,
                    handle_game_exit,
                ),
            );
    }
}
