use std::ops::Deref;
use std::str::FromStr;

use bevy::app::{App, AppExit, Plugin, Update};
use bevy::asset::AssetServer;
use bevy::audio::AudioBundle;
use bevy::ecs::change_detection::{Res, ResMut};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EventWriter;
use bevy::ecs::query::{Changed, With};
use bevy::ecs::schedule::common_conditions::in_state;
use bevy::ecs::schedule::{IntoSystemConfigs, NextState, OnEnter, OnExit, State};
use bevy::ecs::system::{Commands, Query};
use bevy::hierarchy::{BuildChildren, DespawnRecursiveExt};
use bevy::input::{keyboard::KeyCode, mouse::MouseButton, ButtonInput};
use bevy::render::color::Color;
use bevy::tasks::{block_on, futures_lite::future};
use bevy::ui::node_bundles::{ButtonBundle, NodeBundle};
use bevy::ui::widget::Button;
use bevy::ui::{BackgroundColor, Display, GridPlacement, Interaction, Style, UiImage, Val};
use bevy::utils::default;
use bevy_simple_text_input::{TextInputInactive, TextInputPlugin, TextInputValue};
use game_server::game::game::GameState;

use crate::app_state::{AppState, AppStateTransition, MenuState};
use crate::game::{CurrentGame, GameInfo};
use crate::grpc::{CallCreateGame, CallGetPlayerGames, GrpcClient};
use crate::interface::buttons::{
    spawn_exit_button, spawn_join_game_button_bundle, spawn_menu_navigation_button, JoinGame,
};
use crate::interface::common::button_bundle::{
    menu_navigation, menu_navigation_with_associated_text_input, submit_text_input_setting,
};
use crate::interface::common::{
    column_node_bundle, global_column_node_bundle, menu_item_style, menu_text_bundle,
    menu_text_input_bundle, menu_text_style, row_node_bundle, square_ui_image,
    tic_tac_toe_grid_node_bundle, CONFIRMATION_SOUND_PATH, ERROR_SOUND_PATH, O_SPRITE_PATH,
    X_SPRITE_PATH,
};
use crate::interface::components::{AssociatedGameList, AssociatedTextInput};
use crate::interface::game_list::{GameList, GameListBundle, LoadingGameListBundle};
use crate::settings::{Settings, SubmitTextInputSetting};

fn play_sound(commands: &mut Commands, asset_server: &AssetServer, sound_path: &'static str) {
    commands.spawn(AudioBundle {
        source: asset_server.load(sound_path),
        ..default()
    });
}

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TextInputPlugin)
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
                    (handle_player_games_task, join_game)
                        .run_if(in_state(AppState::Menu(MenuState::PlayOverNetwork))),
                    handle_create_game_task,
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

fn join_game(
    join_button: Query<(&Interaction, &JoinGame), (With<Button>, Changed<Interaction>)>,
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    settings: Res<Settings>,
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
                    let game_list_id = parent
                        .spawn(LoadingGameListBundle {
                            container: column_node_bundle(),
                            games: default(),
                            task: CallGetPlayerGames(task),
                        })
                        .id();
                    parent
                        .spawn((
                            ButtonBundle {
                                style: menu_style.clone(),
                                image: UiImage::default(),
                                ..default()
                            },
                            AssociatedGameList(game_list_id),
                        ))
                        .with_children(|parent| {
                            parent.spawn(menu_text_bundle("Refresh", text_style.clone()));
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

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    game: Option<Res<CurrentGame>>,
) {
    let text_style = menu_text_style(&asset_server);

    commands
        .spawn(global_column_node_bundle())
        .with_children(|parent| {
            parent.spawn(row_node_bundle()).with_children(|parent| {
                parent.spawn(menu_text_bundle("Next:", text_style.clone()));
                if let Some(sprite) = game.as_ref().and_then(|game| game.get_next_player_image()) {
                    parent.spawn(square_ui_image(sprite.clone(), Val::Px(50.0)));
                }
            });
            parent
                .spawn(tic_tac_toe_grid_node_bundle())
                .with_children(|parent| {
                    for odd in [1i16, 3, 5] {
                        for even in [2i16, 4] {
                            // vertical borders
                            parent.spawn(NodeBundle {
                                style: Style {
                                    display: Display::Grid,
                                    grid_column: GridPlacement::start(even),
                                    grid_row: GridPlacement::start(odd),
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
                                    grid_column: GridPlacement::start(odd),
                                    grid_row: GridPlacement::start(even),
                                    height: Val::Px(1.0),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::BLACK),
                                ..default()
                            });
                        }
                    }
                });
        });
}

fn handle_player_games_task(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut game_lists: Query<(Entity, &mut GameList, &mut CallGetPlayerGames)>,
) {
    let text_style = menu_text_style(&asset_server);
    let menu_style = menu_item_style();

    for (entity, mut list, mut task) in game_lists.iter_mut() {
        if let Some(res) = block_on(future::poll_once(&mut task.0)) {
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
                                .remove::<CallGetPlayerGames>()
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
                                })
                                .remove::<CallGetPlayerGames>();
                        }
                    }
                }
                Err(err) => {
                    println!("GetPlayerGames request failed: {}", err);
                    commands
                        .entity(entity)
                        .despawn_descendants()
                        .with_children(|parent| {
                            parent.spawn(menu_text_bundle(
                                &format!("Server error: {}", err),
                                text_style.clone(),
                            ));
                        })
                        .remove::<CallGetPlayerGames>();
                }
            }
        }
    }
}

fn handle_create_game_task(
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    asset_server: Res<AssetServer>,
    mut create_game: Query<(Entity, &mut CallCreateGame, &GameInfo)>,
) {
    for (entity, mut task, game) in create_game.iter_mut() {
        if let Some(res) = block_on(future::poll_once(&mut task.0)) {
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
            commands.entity(entity).despawn();
        }
    }
}
