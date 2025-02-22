use std::str::FromStr;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy_simple_text_input::{TextInputInactive, TextInputSubmitEvent, TextInputValue};
use game_server::core::tic_tac_toe::TicTacToe;
use game_server::{core, proto};

use super::common::{
    self, CONFIRMATION_SOUND_PATH, LOGO_HEIGHT, LOGO_WIDTH, MENU_LIST_MIN_HEIGHT, SECONDARY_COLOR,
};
use super::components::{
    CreateGameButtonBundle, GamePageButtonBundle, GameSettingsBundle, ImageBundle, JoinGame, LogIn,
    LogInButtonBundle, LogOut, LogOutButtonBundle, MenuNavigationButtonBundle, Overlay,
    OverlayNodeBundle, Playground, PlaygroundBundle, SettingOption, StorageLink, SubmitButton,
    TextBundle,
};
use super::events::{GameLeft, PlayerGamesReady, SettingOptionPressed, SubmitPressed};
use super::game_list::{GameList, GameListBundle};
use super::resources::RefreshGamesTimer;
use super::{GameReady, GameReadyToExit, GameTag, JoinPressed};
use crate::app_state::{AppState, AppStateTransition, MenuState};
use crate::commands::CommandsExt;
use crate::game::{ActiveGame, Board, CurrentUser, GameInfo, GameLink, GameMenuContext, Winner};
use crate::Settings;
use crate::{grpc, util::watched_value};

/// Whenever game page button is pressed create [`GameMenuContext`] resource and
/// set next state to `AppState::Menu(MenuState::Game)`.
pub fn enter_game_page<T: core::Game + Send + Sync + 'static>(
    mut commands: Commands,
    game_menu_button: Query<&Interaction, (With<Button>, Changed<Interaction>, With<GameTag<T>>)>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    for interaction in game_menu_button.iter() {
        if *interaction == Interaction::Pressed {
            commands.init_resource::<GameMenuContext<T>>();
            next_app_state.set(AppState::Menu(MenuState::Game));
        }
    }
}

/// If current state doesn't related to a specific game remove [`GameMenuContext`] resource.
pub fn remove_game_page_context<T: core::Game + Send + Sync + 'static>(
    mut commands: Commands,
    app_state: Res<State<AppState>>,
) {
    if !matches!(*app_state.get(), AppState::Menu(s) if s.is_game_menu()) {
        commands.remove_resource::<GameMenuContext<T>>();
    }
}

pub fn setup_game_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_font = common::load_text_font(&asset_server);
    let node = common::menu_item_node();

    commands
        .spawn(common::root_node())
        .with_children(|builder| {
            for (state, text) in [
                (AppState::Menu(MenuState::PlayAgainstBot), "Against bot"),
                (AppState::Menu(MenuState::PlayOverNetwork), "Network"),
                (AppState::Menu(MenuState::Main), "Back"),
            ] {
                builder
                    .spawn(MenuNavigationButtonBundle::new(node.clone(), state))
                    .with_child(TextBundle::new(text, text_font.clone()));
            }
        });
}

pub fn toggle_pause(
    mut next_app_state: ResMut<NextState<AppState>>,
    app_state: Res<State<AppState>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if *app_state.get() == AppState::Game {
            next_app_state.set(AppState::Paused);
        } else if *app_state.get() == AppState::Paused {
            next_app_state.set(AppState::Game);
        }
    }
}

pub fn state_transition(
    menu_items: Query<(&Interaction, &AppStateTransition), (With<Button>, Changed<Interaction>)>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
) {
    for (interaction, state_transition) in menu_items.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(new_state) = state_transition.0 {
                info!("state transition: {:?}", new_state);
                next_app_state.set(new_state);
            } else {
                info!("exit");
                exit.send(AppExit::Success);
            }
        }
    }
}

pub fn text_input_focus(
    mut inputs: Query<(&mut TextInputInactive, &Interaction)>,
    button_input: Res<ButtonInput<MouseButton>>,
) {
    if button_input.just_pressed(MouseButton::Left) {
        for (mut inactive, interaction) in inputs.iter_mut() {
            inactive.0 = *interaction != Interaction::Pressed;
        }
    }
}

pub fn join_press(
    join_button: Query<(&Interaction, &JoinGame), (With<Button>, Changed<Interaction>)>,
    mut join_pressed: EventWriter<JoinPressed>,
) {
    if let Some((_, join)) = join_button.iter().find(|(&i, _)| i == Interaction::Pressed) {
        join_pressed.send(JoinPressed::new(join.id));
    }
}

pub fn submit_press(
    submit_button: Query<(&Interaction, &SubmitButton), (With<Button>, Changed<Interaction>)>,
    mut submit_pressed: EventWriter<SubmitPressed>,
) {
    for (interaction, submit) in submit_button.iter() {
        if *interaction == Interaction::Pressed {
            submit_pressed.send(submit.source.into());
        }
    }
}

/// Whenever the button with [`SettingOption`] tag is pressed send [`SettingOptionPressed`] event.
pub fn setting_option_pressed(
    setting_button: Query<
        (Entity, &Interaction),
        (With<Button>, With<SettingOption>, Changed<Interaction>),
    >,
    mut setting_pressed: EventWriter<SettingOptionPressed>,
) {
    for (button_entity, interaction) in setting_button.iter() {
        if *interaction == Interaction::Pressed {
            setting_pressed.send(button_entity.into());
        }
    }
}

/// Receive [`SubmitPressed`] event, find [`TextInputValue`] and try to convert its value to `T`.
/// On success set [`watched_value::WatchedValue`] pointed by [`StorageLink`] of input entity.
pub fn set_local_text_input_setting<T: FromStr + Send + Sync + 'static>(
    mut setting: Query<&mut watched_value::WatchedValue<T>>,
    input: Query<(&TextInputValue, &StorageLink)>,
    mut submit_pressed: EventReader<SubmitPressed>,
    mut text_input_submit: EventReader<TextInputSubmitEvent>,
) {
    let submit_pressed_iter = submit_pressed
        .read()
        .filter_map(|e| input.get(e.get()).ok());
    let text_input_submit_iter = text_input_submit
        .read()
        .filter_map(|e| input.get(e.entity).ok());
    for (input_value, link) in submit_pressed_iter.chain(text_input_submit_iter) {
        let Ok(mut setting) = setting.get_mut(link.get()) else {
            continue;
        };
        if let Ok(value) = input_value.0.parse::<T>() {
            setting.set(value);
        }
    }
}

/// Receive [`SettingOptionPressed`] event, find value of type `T` stored in
/// [`SettingOption`] entity and set [`watched_value::WatchedValue`] pointed by its [`StorageLink`].
pub fn set_local_option_setting<T: Copy + Component>(
    mut setting: Query<&mut watched_value::WatchedValue<T>>,
    source: Query<(&T, &StorageLink), With<SettingOption>>,
    mut setting_pressed: EventReader<SettingOptionPressed>,
) {
    for event in setting_pressed.read() {
        let Ok((value, link)) = source.get(event.get()) else {
            continue;
        };
        let Ok(mut setting) = setting.get_mut(link.get()) else {
            continue;
        };
        setting.set(*value);
    }
}

/// Receive [`watched_value::ValueUpdated`] event and update border color of option buttons
/// that are pointing by [`StorageLink`] to the event source.
/// If the button value is equal to the one in the event set the color to `SECONDARY_COLOR`,
/// otherwise set it to `Color::NONE`.
pub fn update_option_buttons_border<T: PartialEq + Component>(
    mut button: Query<(&mut BorderColor, &T, &StorageLink), With<SettingOption>>,
    mut setting_updated: EventReader<watched_value::ValueUpdated<T>>,
) {
    for event in setting_updated.read() {
        for (mut border_color, button_value, _) in button
            .iter_mut()
            .filter(|(.., link)| link.get() == event.source())
        {
            *border_color = if matches!(event.value(), Some(value) if value == button_value) {
                SECONDARY_COLOR.into()
            } else {
                Color::NONE.into()
            };
        }
    }
}

pub fn cleanup_ui(mut commands: Commands, ui_nodes: Query<Entity, With<Node>>) {
    for entity in ui_nodes.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn setup_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let logo = asset_server.load("sprites/logo.png");
    let text_font = common::load_text_font(&asset_server);
    let item_node = common::menu_item_node();

    commands
        .spawn(common::root_node())
        .with_children(|builder| {
            builder.spawn(ImageBundle::new(
                Node {
                    height: Val::Px(LOGO_HEIGHT),
                    width: Val::Px(LOGO_WIDTH),
                    margin: UiRect::all(Val::Px(20.)),
                    ..default()
                },
                logo,
            ));
            builder
                .spawn(GamePageButtonBundle::<TicTacToe>::new(item_node.clone()))
                .with_child(TextBundle::new("Tic-Tac-Toe", text_font.clone()));
            builder
                .spawn(MenuNavigationButtonBundle::new(
                    item_node.clone(),
                    AppState::Menu(MenuState::Settings),
                ))
                .with_child(TextBundle::new("Settings", text_font.clone()));
            builder
                .spawn(MenuNavigationButtonBundle::exit(item_node))
                .with_child(TextBundle::new("Exit", text_font));
        });
}

pub fn setup_settings_menu(
    mut commands: Commands,
    settings: Res<Settings>,
    asset_server: Res<AssetServer>,
    client: Option<Res<grpc::GrpcClient>>,
) {
    let text_font = common::load_text_font(&asset_server);
    let item_node = common::menu_item_node();

    let connected = client.map_or(false, |c| c.connected());
    let user_id = settings.user_id();
    let user_id_str = user_id.map(|user| user.to_string()).unwrap_or_default();
    commands
        .spawn(common::root_node())
        .with_children(|builder| {
            builder.spawn(common::row_node()).with_children(|builder| {
                if connected {
                    // TODO: fix styles
                    builder.spawn(TextBundle::new("User id:", text_font.clone()));
                    builder.spawn(TextBundle::new(user_id_str, text_font.clone()));
                    if user_id.is_none() {
                        builder
                            .spawn(LogInButtonBundle::new(item_node.clone()))
                            .with_child(TextBundle::new("Log In", text_font.clone()));
                    } else {
                        builder
                            .spawn(LogOutButtonBundle::new(item_node.clone()))
                            .with_child(TextBundle::new("Log Out", text_font.clone()));
                    }
                } else {
                    builder.spawn(TextBundle::new("Offline", text_font.clone()));
                }
            });
            builder
                .spawn(MenuNavigationButtonBundle::new(
                    item_node,
                    AppState::Menu(MenuState::Main),
                ))
                .with_child(TextBundle::new("Back", text_font));
        });
}

pub fn setup_play_against_bot_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_font = common::load_text_font(&asset_server);
    let item_node = common::menu_item_node();

    commands
        .spawn(common::root_node())
        .with_children(|builder| {
            builder.spawn(GameSettingsBundle::new());
            builder
                .spawn(CreateGameButtonBundle::new(item_node.clone()))
                .with_child(TextBundle::new("Play", text_font.clone()));
            builder
                .spawn(MenuNavigationButtonBundle::new(
                    item_node.clone(),
                    AppState::Menu(MenuState::Game),
                ))
                .with_child(TextBundle::new("Back", text_font));
        });
}

pub fn setup_play_over_network_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_font = common::load_text_font(&asset_server);
    let item_node = common::menu_item_node();

    commands
        .spawn(common::root_node())
        .with_children(|builder| {
            builder
                .spawn(common::column_node())
                .with_children(|builder| {
                    builder.spawn(GameSettingsBundle::new());
                    builder
                        .spawn(CreateGameButtonBundle::new(item_node.clone()))
                        .with_child(TextBundle::new("Create game", text_font.clone()));
                });
            let mut game_list = GameListBundle::default();
            game_list.node_mut().min_height = Val::Px(MENU_LIST_MIN_HEIGHT);
            builder.spawn(game_list);
            builder
                .spawn(MenuNavigationButtonBundle::new(
                    item_node.clone(),
                    AppState::Menu(MenuState::Game),
                ))
                .with_child(TextBundle::new("Back", text_font));
        });
}

pub fn setup_pause(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_font = common::load_text_font(&asset_server);
    let item_node = common::menu_item_node();

    commands
        .spawn(OverlayNodeBundle::default())
        .with_children(|builder| {
            builder
                .spawn(common::column_node())
                .with_children(|builder| {
                    for (state, text) in [
                        (AppState::Game, "Resume"),
                        (AppState::Menu(MenuState::Settings), "Settings"),
                        (AppState::Menu(MenuState::Main), "Main menu"),
                    ] {
                        builder
                            .spawn(MenuNavigationButtonBundle::new(item_node.clone(), state))
                            .with_child(TextBundle::new(text, text_font.clone()));
                    }
                });
        });
}

pub fn clear_pause_overlay(mut commands: Commands, overlay: Query<Entity, With<Overlay>>) {
    for overlay in overlay.iter() {
        commands.entity(overlay).despawn_recursive();
    }
}

pub fn setup_game(mut commands: Commands, game: Query<Entity, With<ActiveGame>>) {
    if let Ok(game_entity) = game.get_single() {
        commands.spawn(PlaygroundBundle::new(game_entity));
    } else {
        error!("multiple games found");
    }
}

pub fn start_game(
    mut commands: Commands,
    mut game_ready: EventReader<GameReady>,
    mut next_app_state: ResMut<NextState<AppState>>,
    asset_server: Res<AssetServer>,
) {
    for event in game_ready.read() {
        debug!("starting game: {}", event.get());
        next_app_state.set(AppState::Game);
        commands.entity(event.get()).insert(ActiveGame);
        commands.play_sound(&asset_server, CONFIRMATION_SOUND_PATH);
    }
}

pub fn handle_get_player_games(
    mut get_games_result: EventReader<grpc::RpcResultReady<proto::GetPlayerGamesReply>>,
    mut timer: ResMut<RefreshGamesTimer>,
    mut games_ready: EventWriter<PlayerGamesReady>,
) {
    for event in get_games_result.read() {
        timer.unpause();
        match event.result() {
            Ok(response) => {
                let games: Result<Vec<GameInfo>, _> = response
                    .get_ref()
                    .games
                    .iter()
                    .map(|game| game.clone().try_into())
                    .collect();
                match games {
                    Ok(games) => {
                        games_ready.send(PlayerGamesReady { games: Ok(games) });
                    }
                    Err(err) => {
                        games_ready.send(PlayerGamesReady {
                            games: Err(format!("GetPlayerGames invalid response: {}", err)),
                        });
                    }
                }
            }
            Err(err) => {
                games_ready.send(PlayerGamesReady {
                    games: Err(format!("GetPlayerGames server error: {}", err)),
                });
            }
        }
    }
}

pub fn handle_player_games(
    mut game_list: Query<&mut GameList>,
    mut games_ready: EventReader<PlayerGamesReady>,
) {
    let Some(event) = games_ready.read().last() else {
        return;
    };
    let Ok(mut game_list) = game_list.get_single_mut() else {
        return;
    };
    *game_list = match event.games.clone() {
        Ok(games) => GameList::Games(games),
        Err(msg) => GameList::Message(msg),
    };
}

pub fn create_game_over_overlay(
    mut commands: Commands,
    winner: Query<(Option<&CurrentUser>, &Parent), With<Winner>>,
    mut ready_to_exit: EventReader<GameReadyToExit>,
    asset_server: Res<AssetServer>,
) {
    for event in ready_to_exit.read() {
        let text_font = common::load_text_font(&asset_server);
        let item_node = common::menu_item_node();
        let winner = winner
            .iter()
            .find(|(_, parent)| parent.get() == event.get());
        let text = match winner.map(|val| val.0) {
            Some(Some(_)) => "You win!",
            Some(None) => "You lose!",
            None => "It's a draw!",
        };
        commands
            .spawn(OverlayNodeBundle::default())
            .with_children(|builder| {
                builder
                    .spawn(common::column_node())
                    .with_children(|builder| {
                        builder.spawn(TextBundle::new(
                            &format!("Game over. {}", text),
                            text_font.clone(),
                        ));
                        builder
                            .spawn(MenuNavigationButtonBundle::new(
                                item_node.clone(),
                                AppState::Menu(MenuState::Main),
                            ))
                            .with_child(TextBundle::new("Main menu", text_font));
                    });
            });
    }
}

/// Despawn [`Playground`] and [`Board`] entities and their descendants.
pub fn clear_game_visuals(
    mut commands: Commands,
    playground: Query<(Entity, &GameLink), With<Playground>>,
    board: Query<Entity, With<Board>>,
    mut game_left: EventWriter<GameLeft>,
) {
    for (playground, game_link) in playground.iter() {
        commands.entity(playground).despawn_recursive();
        game_left.send(game_link.get().into());
    }
    for board in board.iter() {
        commands.entity(board).despawn_recursive();
    }
}

/// Remove [`ActiveGame`] component from a game entity.
pub fn deactivate_game(mut commands: Commands, mut game_left: EventReader<GameLeft>) {
    for event in game_left.read() {
        commands.entity(event.get()).remove::<ActiveGame>();
    }
}

/// Whenever log in button is pressed spawn [`grpc::LogInRequest`].
pub fn log_in(
    mut commands: Commands,
    log_in_button: Query<&Interaction, (With<Button>, Changed<Interaction>, With<LogIn>)>,
) {
    for interaction in log_in_button.iter() {
        if *interaction == Interaction::Pressed {
            commands.spawn(grpc::LogInRequest);
        }
    }
}

/// Whenever log out button is pressed send [`grpc::LogOut`] event and clear user id from settings.
pub fn log_out(
    log_out_button: Query<&Interaction, (With<Button>, Changed<Interaction>, With<LogOut>)>,
    mut log_out: EventWriter<grpc::LogOut>,
    mut settings: ResMut<Settings>,
) {
    for interaction in log_out_button.iter() {
        if *interaction == Interaction::Pressed {
            log_out.send(grpc::LogOut);
            settings.reset_user_id();
        }
    }
}
