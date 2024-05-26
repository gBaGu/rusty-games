use std::ops::Deref;
use std::str::FromStr;

use bevy::app::{App, AppExit, Plugin, Update};
use bevy::asset::AssetServer;
use bevy::audio::AudioBundle;
use bevy::ecs::change_detection::{Res, ResMut};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EventWriter;
use bevy::ecs::query::{Changed, With, Without};
use bevy::ecs::schedule::common_conditions::{any_with_component, in_state, not, resource_exists};
use bevy::ecs::schedule::{Condition, IntoSystemConfigs, NextState, OnEnter, OnExit, State};
use bevy::ecs::system::{Commands, Query};
use bevy::hierarchy::{BuildChildren, Children, DespawnRecursiveExt};
use bevy::input::{keyboard::KeyCode, mouse::MouseButton, ButtonInput};
use bevy::prelude::{Deref, DerefMut, Event, EventReader, Resource, Time, TimerMode};
use bevy::render::color::Color;
use bevy::tasks::{block_on, futures_lite::future};
use bevy::time::Timer;
use bevy::ui::node_bundles::NodeBundle;
use bevy::ui::widget::Button;
use bevy::ui::{BackgroundColor, Display, GridPlacement, Interaction, Style, UiImage, Val};
use bevy::utils::default;
use bevy_simple_text_input::{TextInputInactive, TextInputPlugin, TextInputValue};
use game_server::game::game::{BoardCell, FinishedState, GameState};

use crate::app_state::{AppState, AppStateTransition, MenuState};
use crate::game::{CurrentGame, FullGameInfo, GameCellPosition, GameInfo};
use crate::grpc::{CallCreateGame, CallGetGame, CallGetPlayerGames, CallMakeTurn, GrpcClient};
use crate::interface::buttons::{
    spawn_exit_button, spawn_game_cell_button_bundle, spawn_join_game_button_bundle,
    spawn_menu_navigation_button, JoinGame,
};
use crate::interface::common::button_bundle::{
    menu_navigation, menu_navigation_with_associated_text_input, submit_text_input_setting,
};
use crate::interface::common::{
    column_node_bundle, game_button_style, global_column_node_bundle, menu_item_style,
    menu_text_bundle, menu_text_input_bundle, menu_text_style, overlapping_flex_node_bundle,
    row_node_bundle, square_ui_image, tic_tac_toe_grid_node_bundle, CONFIRMATION_SOUND_PATH,
    ERROR_SOUND_PATH, GAME_LIST_REFRESH_INTERVAL_SEC, GAME_REFRESH_INTERVAL_SEC, MENU_ITEM_HEIGHT,
    O_SPRITE_PATH, TURN_SOUND_PATH, X_SPRITE_PATH,
};
use crate::interface::components::{empty_next_player_image, AssociatedTextInput, NextPlayerImage};
use crate::interface::game_list::{GameList, GameListBundle, LoadingGameListBundle};
use crate::settings::{Settings, SubmitTextInputSetting};

fn play_sound(commands: &mut Commands, asset_server: &AssetServer, sound_path: &'static str) {
    commands.spawn(AudioBundle {
        source: asset_server.load(sound_path),
        ..default()
    });
}

#[derive(Deref, DerefMut, Resource)]
struct RefreshGamesTimer(pub Timer);

impl Default for RefreshGamesTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            GAME_LIST_REFRESH_INTERVAL_SEC,
            TimerMode::Repeating,
        ))
    }
}

#[derive(Deref, DerefMut, Resource)]
struct RefreshGameTimer(pub Timer);

impl Default for RefreshGameTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            GAME_REFRESH_INTERVAL_SEC,
            TimerMode::Repeating,
        ))
    }
}

#[derive(Debug, Deref, Event)]
struct GameStateUpdated(GameState);

#[derive(Debug)]
enum CellQueryData {
    Entity(Entity),
    CellPosition { row: usize, col: usize },
}

#[derive(Debug, Event)]
struct CellUpdated {
    query_data: CellQueryData,
    player_id: u64,
}

#[derive(Debug, Deref, Event)]
struct GameOver(FinishedState);

// TODO: replace with actual game with bot
#[derive(Debug, Event)]
struct MockBotGame {
    user_id: u64,
}

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextInputPlugin)
            .add_event::<MockBotGame>()
            .add_event::<GameStateUpdated>()
            .add_event::<CellUpdated>()
            .add_event::<GameOver>()
            .init_resource::<RefreshGamesTimer>()
            .init_resource::<RefreshGameTimer>()
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
            .add_systems(OnExit(AppState::Paused), cleanup_ui)
            .add_systems(OnEnter(AppState::Game), setup_game)
            .add_systems(OnExit(AppState::Game), cleanup_ui)
            .add_systems(
                Update,
                (
                    state_transition,
                    text_input_focus,
                    settings_submit::<u64>.run_if(in_state(AppState::Menu(MenuState::Settings))),
                    (refresh_game_list, handle_player_games_task, join_game)
                        .run_if(in_state(AppState::Menu(MenuState::PlayOverNetwork))),
                    handle_create_game_task,
                    handle_make_turn_task,
                    handle_get_game_task,
                    (
                        make_turn,
                        refresh_game_board.run_if(not(any_with_component::<CallGetGame>)),
                        handle_cell_updated,
                        handle_state_updated,
                        handle_game_over.after(handle_state_updated),
                    )
                        .run_if(in_state(AppState::Game).and_then(resource_exists::<CurrentGame>)),
                    create_mock_bot_game
                        .after(state_transition)
                        .run_if(in_state(AppState::Menu(MenuState::PlayWithBot))),
                ),
            );
    }
}

fn state_transition(
    menu_items: Query<
        (
            &Interaction,
            &AppStateTransition,
            Option<&AssociatedTextInput>,
        ),
        (With<Button>, Changed<Interaction>),
    >,
    text_inputs: Query<(Entity, &TextInputValue)>,
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
    mut mock_bot_game: EventWriter<MockBotGame>,
    app_state: Res<State<AppState>>,
    settings: Res<Settings>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
    grpc_client: Res<GrpcClient>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        if *app_state.get() == AppState::Game {
            next_app_state.set(AppState::Paused);
            return;
        } else if *app_state.get() == AppState::Paused {
            next_app_state.set(AppState::Game);
            return;
        }
    }
    for (interaction, state_transition, associated_input) in menu_items.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(new_state) = state_transition.0 {
                // if transition is AppState::Game and button have associated text input
                // this means it's a network game and input contains opponent id
                // TODO: find a way to express this with types
                if let (AppState::Game, Some(associated_input)) = (new_state, associated_input) {
                    if let Some((_, val)) =
                        text_inputs.iter().find(|(e, _)| *e == associated_input.0)
                    {
                        if let Ok(opponent_id) = val.0.parse::<u64>() {
                            if let Some(user_id) = settings.user_id() {
                                if let Some(task) = grpc_client.create_game(user_id, opponent_id) {
                                    let game = GameInfo {
                                        id: 0, // TODO: consider making optional to show that game is not created yet
                                        state: GameState::Turn(user_id),
                                        players: vec![user_id, opponent_id],
                                    };
                                    commands.spawn((CallCreateGame(task), game));
                                }
                            }
                        } else {
                            play_sound(&mut commands, &asset_server, ERROR_SOUND_PATH);
                        }
                    }
                } else if *app_state.get() == AppState::Menu(MenuState::PlayWithBot)
                    && new_state == AppState::Game
                {
                    if let Some(user_id) = settings.user_id() {
                        mock_bot_game.send(MockBotGame { user_id });
                        println!("state transition: {:?}", new_state);
                        next_app_state.set(new_state);
                    } else {
                        play_sound(&mut commands, &asset_server, ERROR_SOUND_PATH);
                    }
                } else {
                    // regular state transition
                    println!("state transition: {:?}", new_state);
                    next_app_state.set(new_state);
                }
            } else {
                println!("exit");
                exit.send(AppExit);
            }
        }
    }
}

fn create_mock_bot_game(
    mut commands: Commands,
    mut game_over: EventWriter<GameOver>,
    mut mock_bot_game: EventReader<MockBotGame>,
    asset_server: Res<AssetServer>,
) {
    if let Some(&MockBotGame { user_id }) = mock_bot_game.read().last() {
        let x_img = asset_server.load(X_SPRITE_PATH);
        let o_img = asset_server.load(O_SPRITE_PATH);
        commands.insert_resource(CurrentGame::new(
            user_id,
            GameInfo {
                id: 0,
                state: GameState::Finished(FinishedState::Draw),
                players: vec![user_id],
            },
            x_img,
            o_img,
        ));
        game_over.send(GameOver(FinishedState::Draw));
    }
}

fn join_game(
    join_button: Query<(&Interaction, &JoinGame), (With<Button>, Changed<Interaction>)>,
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    settings: Res<Settings>,
    grpc_client: Res<GrpcClient>,
    asset_server: Res<AssetServer>,
) {
    let user_id = match settings.user_id() {
        Some(id) => id,
        None => {
            println!("user_id is not set");
            return;
        }
    };
    if let Some((_, join)) = join_button.iter().find(|(&i, _)| i == Interaction::Pressed) {
        let x_img = asset_server.load(X_SPRITE_PATH);
        let o_img = asset_server.load(O_SPRITE_PATH);
        commands.insert_resource(CurrentGame::new(
            user_id,
            join.deref().clone(),
            x_img,
            o_img,
        ));
        if let Some(task) = grpc_client.load_game(join.id) {
            commands.spawn(CallGetGame(task));
        } else {
            println!("load_game failed");
        }
        println!("state transition: join game");
        next_app_state.set(AppState::Game);
        play_sound(&mut commands, &asset_server, CONFIRMATION_SOUND_PATH);
    }
}

fn text_input_focus(
    mut inputs: Query<(&mut TextInputInactive, &Interaction)>,
    button_input: Res<ButtonInput<MouseButton>>,
) {
    if button_input.just_pressed(MouseButton::Left) {
        for (mut inactive, interaction) in inputs.iter_mut() {
            inactive.0 = *interaction != Interaction::Pressed;
        }
    }
}

fn settings_submit<T: FromStr + 'static>(
    submit_buttons: Query<
        (&Interaction, &SubmitTextInputSetting<T>),
        (With<Button>, Changed<Interaction>),
    >,
    text_inputs: Query<(Entity, &TextInputValue)>,
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    asset_server: Res<AssetServer>,
) {
    for (interaction, submit_input) in submit_buttons.iter() {
        if *interaction == Interaction::Pressed {
            if let Some((_, val)) = text_inputs
                .iter()
                .find(|(e, _)| *e == submit_input.associated_input())
            {
                if let Ok(val) = val.0.parse::<T>() {
                    submit_input.submit(&mut settings, val);
                    play_sound(&mut commands, &asset_server, CONFIRMATION_SOUND_PATH);
                } else {
                    play_sound(&mut commands, &asset_server, ERROR_SOUND_PATH);
                }
            }
        }
    }
}

fn cleanup_ui(mut commands: Commands, ui_nodes: Query<Entity, With<bevy::ui::Node>>) {
    for entity in ui_nodes.iter() {
        commands.entity(entity).despawn();
    }
}

fn setup_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);
    let menu_style = menu_item_style();

    commands
        .spawn(global_column_node_bundle())
        .with_children(|parent| {
            spawn_menu_navigation_button(
                parent,
                menu_style.clone(),
                text_style.clone(),
                "Play",
                AppState::Menu(MenuState::PlayWithBot),
            );
            spawn_menu_navigation_button(
                parent,
                menu_style.clone(),
                text_style.clone(),
                "Network",
                AppState::Menu(MenuState::PlayOverNetwork),
            );
            spawn_menu_navigation_button(
                parent,
                menu_style.clone(),
                text_style.clone(),
                "Settings",
                AppState::Menu(MenuState::Settings),
            );
            spawn_exit_button(parent, menu_style, text_style, "Exit");
        });
}

fn setup_settings_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);
    let menu_style = menu_item_style();

    commands
        .spawn(global_column_node_bundle())
        .with_children(|parent| {
            parent.spawn(row_node_bundle()).with_children(|parent| {
                parent.spawn(menu_text_bundle("Set user id:", text_style.clone()));
                let input_id = parent
                    .spawn(menu_text_input_bundle(
                        text_style.clone(),
                        menu_style.clone(),
                    ))
                    .id();
                parent
                    .spawn(submit_text_input_setting(
                        menu_style.clone(),
                        input_id,
                        Settings::set_user_id,
                    ))
                    .with_children(|parent| {
                        parent.spawn(menu_text_bundle("Save", text_style.clone()));
                    });
            });
            parent
                .spawn(menu_navigation(
                    menu_style.clone(),
                    AppState::Menu(MenuState::Main),
                ))
                .with_children(|parent| {
                    parent.spawn(menu_text_bundle("Back", text_style.clone()));
                });
        });
}

fn setup_play_with_bot_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);
    let menu_style = menu_item_style();

    commands
        .spawn(global_column_node_bundle())
        .with_children(|parent| {
            parent
                .spawn(menu_navigation(menu_style.clone(), AppState::Game))
                .with_children(|parent| {
                    parent.spawn(menu_text_bundle("Play with bot", text_style.clone()));
                });
            parent
                .spawn(menu_navigation(
                    menu_style.clone(),
                    AppState::Menu(MenuState::Main),
                ))
                .with_children(|parent| {
                    parent.spawn(menu_text_bundle("Back", text_style.clone()));
                });
        });
}

fn setup_play_over_network_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
    grpc_client: Res<GrpcClient>,
) {
    let text_style = menu_text_style(&asset_server);
    let menu_style = menu_item_style();

    commands
        .spawn(global_column_node_bundle())
        .with_children(|parent| {
            parent.spawn(row_node_bundle()).with_children(|parent| {
                parent.spawn(menu_text_bundle("Opponent id:", text_style.clone()));
                let input_id = parent
                    .spawn(menu_text_input_bundle(
                        text_style.clone(),
                        menu_style.clone(),
                    ))
                    .id();
                parent
                    .spawn(menu_navigation_with_associated_text_input(
                        menu_style.clone(),
                        AppState::Game,
                        input_id,
                    ))
                    .with_children(|parent| {
                        parent.spawn(menu_text_bundle("Create game", text_style.clone()));
                    });
            });
            if let Some(id) = settings.user_id() {
                if let Some(task) = grpc_client.load_player_games(id) {
                    parent.spawn(LoadingGameListBundle {
                        container: column_node_bundle(),
                        games: default(),
                        task: CallGetPlayerGames(task),
                    });
                } else {
                    parent
                        .spawn(GameListBundle {
                            container: column_node_bundle(),
                            games: default(),
                        })
                        .with_children(|parent| {
                            parent.spawn(menu_text_bundle("Server is down", text_style.clone()));
                        });
                }
            } else {
                parent
                    .spawn(GameListBundle {
                        container: column_node_bundle(),
                        games: default(),
                    })
                    .with_children(|parent| {
                        parent.spawn(menu_text_bundle("No user id provided", text_style.clone()));
                    });
            }
            parent
                .spawn(menu_navigation(
                    menu_style.clone(),
                    AppState::Menu(MenuState::Main),
                ))
                .with_children(|parent| {
                    parent.spawn(menu_text_bundle("Back", text_style.clone()));
                });
        });
}

fn setup_pause(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);
    let menu_style = menu_item_style();

    commands
        .spawn(global_column_node_bundle())
        .with_children(|parent| {
            parent
                .spawn(menu_navigation(menu_style.clone(), AppState::Game))
                .with_children(|parent| {
                    parent.spawn(menu_text_bundle("Resume", text_style.clone()));
                });
            parent
                .spawn(menu_navigation(
                    menu_style.clone(),
                    AppState::Menu(MenuState::Settings),
                ))
                .with_children(|parent| {
                    parent.spawn(menu_text_bundle("Settings", text_style.clone()));
                });
            parent
                .spawn(menu_navigation(
                    menu_style.clone(),
                    AppState::Menu(MenuState::Main),
                ))
                .with_children(|parent| {
                    parent.spawn(menu_text_bundle("Main menu", text_style.clone()));
                });
        });
}

fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);

    commands
        .spawn(global_column_node_bundle())
        .with_children(|parent| {
            parent.spawn(row_node_bundle()).with_children(|parent| {
                parent.spawn(menu_text_bundle("Next:", text_style.clone()));
                parent.spawn(empty_next_player_image(Val::Px(MENU_ITEM_HEIGHT)));
            });
            parent
                .spawn(tic_tac_toe_grid_node_bundle())
                .with_children(|parent| {
                    for i in 1i16..=5 {
                        for j in 1i16..=5 {
                            let i_is_odd = i % 2 != 0;
                            let j_is_odd = j % 2 != 0;
                            if i_is_odd && j_is_odd {
                                // spawn game buttons
                                let position =
                                    GameCellPosition::new((i / 2) as u32, (j / 2) as u32);
                                let style = game_button_style(i, j);
                                spawn_game_cell_button_bundle(parent, style, position);
                            }
                            if i_is_odd && !j_is_odd {
                                // vertical borders
                                parent.spawn(NodeBundle {
                                    style: Style {
                                        display: Display::Grid,
                                        grid_column: GridPlacement::start(j),
                                        grid_row: GridPlacement::start(i),
                                        width: Val::Px(1.0),
                                        ..default()
                                    },
                                    background_color: BackgroundColor(Color::BLACK),
                                    ..default()
                                });
                                // horizontal borders
                                parent.spawn(NodeBundle {
                                    style: Style {
                                        display: Display::Grid,
                                        grid_column: GridPlacement::start(i),
                                        grid_row: GridPlacement::start(j),
                                        height: Val::Px(1.0),
                                        ..default()
                                    },
                                    background_color: BackgroundColor(Color::BLACK),
                                    ..default()
                                });
                            }
                        }
                    }
                });
        });
}

fn make_turn(
    game_cell: Query<
        (Entity, &Interaction, &GameCellPosition),
        (With<Button>, Without<Children>, Changed<Interaction>),
    >,
    mut commands: Commands,
    game: Res<CurrentGame>,
    asset_server: Res<AssetServer>,
    client: ResMut<GrpcClient>,
) {
    if let Some((entity, _, &pos)) = game_cell
        .iter()
        .find(|(_, &i, _)| i == Interaction::Pressed)
    {
        if let Some(task) = client.make_turn(game.id(), game.user_id(), pos) {
            commands.entity(entity).insert(CallMakeTurn(task));
        } else {
            println!("grpc server is down");
            play_sound(&mut commands, &asset_server, ERROR_SOUND_PATH);
        }
    }
}

fn refresh_game_list(
    game_lists: Query<Entity, (With<GameList>, Without<CallGetPlayerGames>)>,
    mut commands: Commands,
    mut timer: ResMut<RefreshGamesTimer>,
    client: ResMut<GrpcClient>,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
    time: Res<Time>,
) {
    if timer.tick(time.delta()).just_finished() {
        let text_style = menu_text_style(&asset_server);
        for entity in game_lists.iter() {
            if let Some(id) = settings.user_id() {
                if let Some(task) = client.load_player_games(id) {
                    commands.entity(entity).insert(CallGetPlayerGames(task));
                } else {
                    commands
                        .entity(entity)
                        .despawn_descendants()
                        .with_children(|parent| {
                            parent.spawn(menu_text_bundle("Server is down", text_style.clone()));
                        });
                }
            } else {
                commands
                    .entity(entity)
                    .despawn_descendants()
                    .with_children(|parent| {
                        parent.spawn(menu_text_bundle("No user id provided", text_style.clone()));
                    });
            }
        }
    }
}

fn refresh_game_board(
    mut commands: Commands,
    mut timer: ResMut<RefreshGamesTimer>,
    game: Res<CurrentGame>,
    client: ResMut<GrpcClient>,
    time: Res<Time>,
) {
    if timer.tick(time.delta()).just_finished() {
        if let Some(task) = client.load_game(game.id()) {
            commands.spawn(CallGetGame(task));
        }
    }
}

fn handle_player_games_task(
    mut game_lists: Query<(Entity, &mut GameList, &mut CallGetPlayerGames)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let text_style = menu_text_style(&asset_server);
    let menu_style = menu_item_style();

    for (entity, mut list, mut task) in game_lists.iter_mut() {
        if let Some(res) = block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).remove::<CallGetPlayerGames>();
            match res {
                Ok(response) => {
                    let games: Result<Vec<GameInfo>, _> = response
                        .into_inner()
                        .games
                        .into_iter()
                        .map(|game| game.try_into())
                        .collect();
                    match games {
                        Ok(games) => {
                            list.games = games.clone();
                            commands
                                .entity(entity)
                                .despawn_descendants()
                                .with_children(|parent| {
                                    if games.is_empty() {
                                        parent.spawn(menu_text_bundle(
                                            "No games available",
                                            text_style.clone(),
                                        ));
                                        return;
                                    }
                                    for game in games.iter() {
                                        parent.spawn(row_node_bundle()).with_children(|parent| {
                                            parent.spawn(menu_text_bundle(
                                                &format!("ID: {}", game.id),
                                                text_style.clone(),
                                            ));
                                            parent.spawn(menu_text_bundle(
                                                &format!("{:?}", game.state),
                                                text_style.clone(),
                                            ));
                                            parent.spawn(menu_text_bundle(
                                                &format!("{:?}", game.players),
                                                text_style.clone(),
                                            ));
                                            spawn_join_game_button_bundle(
                                                parent,
                                                menu_style.clone(),
                                                text_style.clone(),
                                                "Join",
                                                game.clone(),
                                            );
                                        });
                                    }
                                });
                        }
                        Err(err) => {
                            println!("GetPlayerGames invalid response: {}", err);
                            commands
                                .entity(entity)
                                .despawn_descendants()
                                .with_children(|parent| {
                                    parent.spawn(menu_text_bundle(
                                        &format!("Server error: {}", err),
                                        text_style.clone(),
                                    ));
                                });
                        }
                    }
                }
                Err(err) => {
                    commands
                        .entity(entity)
                        .despawn_descendants()
                        .with_children(|parent| {
                            parent.spawn(menu_text_bundle(
                                &format!("Server error: {}", err.code().description()),
                                text_style.clone(),
                            ));
                        });
                }
            }
        }
    }
}

fn handle_create_game_task(
    mut create_game: Query<(Entity, &mut CallCreateGame, &GameInfo)>,
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut state_updated: EventWriter<GameStateUpdated>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut task, game) in create_game.iter_mut() {
        if let Some(res) = block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).despawn();
            match res {
                Ok(response) => {
                    let game_id = response.into_inner().game_id;
                    if let Some(&user_id) = game.players.first() {
                        let x_img = asset_server.load(X_SPRITE_PATH);
                        let o_img = asset_server.load(O_SPRITE_PATH);
                        commands.insert_resource(CurrentGame::new(
                            user_id,
                            GameInfo {
                                id: game_id,
                                players: game.players.clone(),
                                state: game.state,
                            },
                            x_img,
                            o_img,
                        ));
                        println!("starting created game: {}", game_id);
                        state_updated.send(GameStateUpdated(game.state));
                        next_app_state.set(AppState::Game);
                        play_sound(&mut commands, &asset_server, CONFIRMATION_SOUND_PATH);
                    } else {
                        println!("GameInfo is corrupted: players is empty");
                        play_sound(&mut commands, &asset_server, ERROR_SOUND_PATH);
                    }
                }
                Err(err) => {
                    println!("CreateGame request failed: {}", err);
                    play_sound(&mut commands, &asset_server, ERROR_SOUND_PATH);
                }
            };
        }
    }
}

fn handle_make_turn_task(
    mut make_turn: Query<(Entity, &mut CallMakeTurn, &GameCellPosition)>,
    mut commands: Commands,
    mut game: Option<ResMut<CurrentGame>>,
    mut cell_updated: EventWriter<CellUpdated>,
    mut state_updated: EventWriter<GameStateUpdated>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut task, pos) in make_turn.iter_mut() {
        if let Some(res) = block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).remove::<CallMakeTurn>();
            let game = match &mut game {
                Some(game) => game,
                None => {
                    println!("no current game, dropping MakeTurn response");
                    continue;
                }
            };
            let user_id = game.user_id();
            match res {
                Ok(response) => {
                    if let Some(next) = game.get_next_player() {
                        if next != user_id {
                            println!("state is already updated, skip handling MakeTurn response");
                            continue;
                        }
                    }
                    if let Some(new_state) = response.into_inner().game_state {
                        let state = new_state.try_into();
                        match state {
                            Ok(state) => {
                                println!("updating state on successful turn");
                                game.set_cell((pos.row as usize, pos.col as usize), user_id);
                                game.set_state(state);
                                cell_updated.send(CellUpdated {
                                    query_data: CellQueryData::Entity(entity),
                                    player_id: user_id,
                                });
                                state_updated.send(GameStateUpdated(state));
                                play_sound(&mut commands, &asset_server, TURN_SOUND_PATH);
                            }
                            Err(err) => {
                                println!("failed to convert game state: {}", err);
                            }
                        }
                    } else {
                        println!("MakeTurn returned empty response");
                        play_sound(&mut commands, &asset_server, ERROR_SOUND_PATH);
                    }
                }
                Err(err) => {
                    println!("MakeTurn request failed: {}", err);
                    play_sound(&mut commands, &asset_server, ERROR_SOUND_PATH);
                }
            }
        }
    }
}

fn handle_get_game_task(
    mut get_game: Query<(Entity, &mut CallGetGame)>,
    mut commands: Commands,
    mut game: Option<ResMut<CurrentGame>>,
    mut cell_updated: EventWriter<CellUpdated>,
    mut state_updated: EventWriter<GameStateUpdated>,
) {
    for (entity, mut task) in get_game.iter_mut() {
        if let Some(res) = block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).remove::<CallGetGame>();
            let game = match &mut game {
                Some(game) => game,
                None => {
                    println!("no current game, dropping GetGame response");
                    continue;
                }
            };
            match res {
                Ok(response) => {
                    if let Some(info) = response.into_inner().game_info {
                        if info.game_id != game.id() {
                            println!("received game info for other game, dropping");
                            continue;
                        }
                        let full_info = match FullGameInfo::try_from(info) {
                            Ok(info) => info,
                            Err(err) => {
                                println!("failed to decode full game info: {}", err);
                                continue;
                            }
                        };
                        for (i, row) in full_info.board.iter().enumerate() {
                            for (j, cell) in row.iter().enumerate() {
                                if let BoardCell(Some(player)) = cell {
                                    let local_cell = game.board()[i][j];
                                    if local_cell.is_none() {
                                        game.set_state(full_info.info.state);
                                        game.set_cell((i, j), *player);
                                        state_updated.send(GameStateUpdated(full_info.info.state));
                                        cell_updated.send(CellUpdated {
                                            query_data: CellQueryData::CellPosition {
                                                row: i,
                                                col: j,
                                            },
                                            player_id: *player,
                                        });
                                    }
                                }
                            }
                        }
                    } else {
                        println!("GetGame returned empty response");
                    }
                }
                Err(err) => {
                    println!("GetGame request failed: {}", err);
                }
            }
        }
    }
}

fn handle_cell_updated(
    buttons: Query<(Entity, &GameCellPosition), With<Button>>,
    mut commands: Commands,
    mut cell_updated: EventReader<CellUpdated>,
    game: Res<CurrentGame>,
) {
    for event in cell_updated.read() {
        if let Some(img) = game.get_player_image(&event.player_id) {
            let entity = match event.query_data {
                CellQueryData::Entity(entity) => entity,
                CellQueryData::CellPosition { row, col } => {
                    match buttons
                        .iter()
                        .find(|(_, pos)| pos.row as usize == row && pos.col as usize == col)
                    {
                        Some((entity, _)) => entity,
                        None => {
                            println!("unable to get button with position: ({}, {})", row, col);
                            continue;
                        }
                    }
                }
            };
            commands.entity(entity).with_children(|parent| {
                parent.spawn(square_ui_image(img.clone(), Val::Percent(100.0)));
            });
        }
    }
}

fn handle_state_updated(
    mut next_player_image: Query<
        (Entity, &mut BackgroundColor, Option<&mut UiImage>),
        With<NextPlayerImage>,
    >,
    mut commands: Commands,
    mut state_updated: EventReader<GameStateUpdated>,
    mut game_over: EventWriter<GameOver>,
    game: Res<CurrentGame>,
) {
    if let Some(event) = state_updated.read().last() {
        println!("received GameStateUpdated: {:?}", event);
        match event.0 {
            GameState::Turn(id) => {
                if let Some(&ref next_img) = game.get_player_image(&id) {
                    match next_player_image.get_single_mut() {
                        Ok((_, _, Some(mut img))) => *img = UiImage::new(next_img.clone()),
                        Ok((entity, mut bc, None)) => {
                            *bc = BackgroundColor(Color::WHITE);
                            commands
                                .entity(entity)
                                .insert(UiImage::new(next_img.clone()));
                        }
                        Err(err) => println!("failed to get NextPlayerImage: {}", err),
                    };
                } else {
                    println!("failed to get image for player: {}", id);
                }
            }
            GameState::Finished(f) => {
                game_over.send(GameOver(f));
            }
        };
    }
}

fn handle_game_over(
    mut commands: Commands,
    mut game_over: EventReader<GameOver>,
    game: Res<CurrentGame>,
    asset_server: Res<AssetServer>,
) {
    if let Some(event) = game_over.read().last() {
        let text_style = menu_text_style(&asset_server);
        let style = menu_item_style();
        let text = match event.deref() {
            FinishedState::Win(id) => {
                if *id == game.user_id() {
                    "You win!"
                } else {
                    "You lose!"
                }
            }
            FinishedState::Draw => "It's a draw!",
        };
        commands
            .spawn(overlapping_flex_node_bundle())
            .with_children(|parent| {
                parent.spawn(column_node_bundle()).with_children(|parent| {
                    parent.spawn(menu_text_bundle(
                        &format!("Game over. {}", text),
                        text_style.clone(),
                    ));
                    spawn_menu_navigation_button(
                        parent,
                        style,
                        text_style,
                        "Main menu",
                        AppState::Menu(MenuState::Main),
                    );
                });
            });
    }
}
