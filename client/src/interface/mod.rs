pub mod common;
pub mod systems;

mod components;
mod game_list;
mod ingame;
mod resources;
mod events;

use bevy::prelude::*;
use bevy_simple_text_input::TextInputPlugin;

use crate::app_state::{AppState, MenuState};
use crate::game::CurrentGame;
use events::SubmitPressed;
use ingame::InGameUIPlugin;
use resources::RefreshGamesTimer;
use systems::*;

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((InGameUIPlugin, TextInputPlugin))
            .init_resource::<RefreshGamesTimer>()
            .add_event::<SubmitPressed>()
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
            .add_systems(OnExit(AppState::Paused), exit_pause)
            .add_systems(
                OnEnter(AppState::Game),
                setup_game.run_if(not(any_with_component::<Node>)),
            )
            .add_systems(
                OnExit(AppState::Game),
                cleanup_ui.run_if(not(in_state(AppState::Paused))),
            )
            .add_systems(
                Update,
                (
                    state_transition,
                    submit_press,
                    text_input_focus,
                    settings_submit::<u64>.run_if(in_state(AppState::Menu(MenuState::Settings))),
                    (refresh_game_list, handle_player_games_task, create_game, join_game)
                        .run_if(in_state(AppState::Menu(MenuState::PlayOverNetwork))),
                    handle_create_game_task,
                    (
                        make_turn,
                        handle_cell_updated,
                        handle_game_over,
                        handle_successful_turn,
                    )
                        .run_if(in_state(AppState::Game).and_then(resource_exists::<CurrentGame>)),
                ),
            );
    }
}
