mod components;
mod systems;

use bevy::prelude::*;
use game_server::core::tic_tac_toe::TicTacToe;

use crate::app_state::{AppState, MenuState};
use crate::game::GameMenuContext;
use crate::grpc::NetworkSystems;
use systems::*;

pub struct GameMenuPlugin;

impl Plugin for GameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (init_bot_settings_menu, create_bot_game)
                    .run_if(in_state(AppState::Menu(MenuState::PlayAgainstBot))),
                (init_network_settings_menu, create_network_game)
                    .run_if(in_state(AppState::Menu(MenuState::PlayOverNetwork))),
                init_game_list,
                send_get_player_games.in_set(NetworkSystems),
                update_active_strategy,
                update_strategy_button_border,
                update_active_difficulty,
                save_opponent,
                join_game,
            )
                .run_if(resource_exists::<GameMenuContext<TicTacToe>>),
        );
    }
}
