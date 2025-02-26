pub mod common;

mod components;
mod events;
mod game_list;
mod resources;
mod systems;

use bevy::prelude::*;
use bevy_simple_text_input::TextInputPlugin;

use crate::app_state::{AppState, MenuState};
use crate::grpc;
use events::PlayerGamesReady;
use systems::*;

pub use components::{
    CreateGame, GameSettingsContainer, GameTag, JoinGameButtonBundle, PlayerColor, Playground,
    SettingOptionButtonBundle, StorageLink, SubmitButtonBundle, TextBundle, TextInputBundle,
};
pub use events::{
    GameLeft, GameReady, GameReadyToExit, JoinPressed, SettingOptionPressed, SubmitPressed,
};
pub use game_list::GameList;
pub use resources::RefreshGamesTimer;
pub use systems::{
    enter_game_page, remove_game_page_context, set_local_option_setting,
    update_option_buttons_border,
};

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextInputPlugin)
            .init_resource::<RefreshGamesTimer>()
            .add_event::<GameReady>()
            .add_event::<GameReadyToExit>()
            .add_event::<GameLeft>()
            .add_event::<SubmitPressed>()
            .add_event::<SettingOptionPressed>()
            .add_event::<JoinPressed>()
            .add_event::<PlayerGamesReady>()
            .add_systems(OnEnter(AppState::Menu(MenuState::Main)), setup_main_menu)
            .add_systems(OnExit(AppState::Menu(MenuState::Main)), cleanup_ui)
            .add_systems(OnEnter(AppState::Menu(MenuState::Game)), setup_game_menu)
            .add_systems(OnExit(AppState::Menu(MenuState::Game)), cleanup_ui)
            .add_systems(
                OnEnter(AppState::Menu(MenuState::Settings)),
                setup_settings_menu,
            )
            .add_systems(OnExit(AppState::Menu(MenuState::Settings)), cleanup_ui)
            .add_systems(
                OnEnter(AppState::Menu(MenuState::PlayAgainstBot)),
                setup_play_against_bot_menu,
            )
            .add_systems(
                OnExit(AppState::Menu(MenuState::PlayAgainstBot)),
                cleanup_ui,
            )
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
                    clear_game_visuals.run_if(not(in_state(AppState::Game))),
                ),
            )
            .add_systems(
                OnEnter(AppState::Game),
                setup_game.run_if(not(any_with_component::<Playground>)),
            )
            .add_systems(
                OnExit(AppState::Game),
                (cleanup_ui, clear_game_visuals.before(cleanup_ui))
                    .run_if(not(in_state(AppState::Paused))),
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
                    setting_option_pressed,
                    set_local_text_input_setting::<u64>,
                    text_input_focus,
                    (
                        join_press.in_set(grpc::NetworkSystems),
                        handle_get_player_games,
                        handle_player_games,
                    )
                        .run_if(in_state(AppState::Menu(MenuState::PlayOverNetwork))),
                    create_game_over_overlay,
                    deactivate_game,
                    log_in,
                    log_out,
                    update_user_id,
                    show_log_in_button,
                    show_log_out_button,
                ),
            );
    }
}
