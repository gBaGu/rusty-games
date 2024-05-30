use std::ops::Deref;
use std::str::FromStr;

use bevy::app::{App, AppExit, Plugin, Update};
use bevy::asset::AssetServer;
use bevy::ecs::change_detection::{Res, ResMut};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::EventWriter;
use bevy::ecs::query::{Changed, With, Without};
use bevy::ecs::schedule::common_conditions::{any_with_component, in_state, not, resource_exists};
use bevy::ecs::schedule::{Condition, IntoSystemConfigs, NextState, OnEnter, OnExit, State};
use bevy::ecs::system::{Commands, Query};
use bevy::hierarchy::{BuildChildren, DespawnRecursiveExt};
use bevy::input::{keyboard::KeyCode, mouse::MouseButton, ButtonInput};
use bevy::prelude::{Deref, DerefMut, EventReader, Resource};
use bevy::tasks::{block_on, futures_lite::future};
use bevy::time::{Time, Timer, TimerMode};
use bevy::ui::widget::Button;
use bevy::ui::Interaction;
use bevy::utils::default;
use bevy_simple_text_input::{TextInputInactive, TextInputPlugin, TextInputValue};
use game_server::game::game::{FinishedState, GameState};

use crate::app_state::{AppState, AppStateTransition, MenuState};
use crate::board;
use crate::commands::CommandsExt;
use crate::game::{
    CellUpdated, CurrentGame, GameInfo, GameOver, GamePlugin, StateUpdated, SuccessfulTurn,
};
use crate::grpc::{CallCreateGame, CallGetGame, CallGetPlayerGames, CallMakeTurn, GrpcClient};
use crate::interface::buttons::{
    spawn_exit_button, spawn_join_game_button_bundle, spawn_menu_navigation_button, JoinGame,
};
use crate::interface::common::button_bundle::{
    menu_navigation, menu_navigation_with_associated_text_input, submit_text_input_setting,
};
use crate::interface::common::{
    column_node_bundle, global_column_node_bundle, menu_item_style, menu_text_bundle,
    menu_text_input_bundle, menu_text_style, row_node_bundle, CONFIRMATION_SOUND_PATH,
    ERROR_SOUND_PATH, GAME_LIST_REFRESH_INTERVAL_SEC, TURN_SOUND_PATH,
};
use crate::interface::components::{overlay_ui_node, AssociatedTextInput, Overlay};
use crate::interface::game_list::{GameList, GameListBundle, LoadingGameListBundle};
use crate::interface::ingame::{InGameUIBundle, InGameUIPlugin, PlayerInfoReady};
use crate::settings::{Settings, SubmitTextInputSetting};

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

pub struct InterfacePlugin;

impl Plugin for InterfacePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((GamePlugin, InGameUIPlugin, TextInputPlugin))
            .init_resource::<RefreshGamesTimer>()
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
                setup_game.run_if(not(any_with_component::<bevy::ui::Node>)),
            )
            .add_systems(
                OnExit(AppState::Game),
                cleanup_ui.run_if(not(in_state(AppState::Paused))),
            )
            .add_systems(
                Update,
                (
                    state_transition,
                    text_input_focus,
                    settings_submit::<u64>.run_if(in_state(AppState::Menu(MenuState::Settings))),
                    (refresh_game_list, handle_player_games_task, join_game)
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
                                    commands.spawn(CallCreateGame(task));
                                }
                            }
                        } else {
                            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                        }
                    }
                } else if *app_state.get() == AppState::Menu(MenuState::PlayWithBot)
                    && new_state == AppState::Game
                {
                    if let Some(user_id) = settings.user_id() {
                        let game = CurrentGame::new_from_asset_server(
                            user_id,
                            GameInfo {
                                id: 0,
                                state: GameState::Finished(FinishedState::Draw),
                                players: vec![user_id, u64::MAX], // TODO: replace this with bot id
                            },
                            &asset_server,
                        );
                        commands.insert_resource(game);
                        println!("state transition: {:?}", new_state);
                        next_app_state.set(new_state);
                    } else {
                        commands.play_sound(&asset_server, ERROR_SOUND_PATH);
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
        commands.insert_resource(CurrentGame::new_from_asset_server(
            user_id,
            join.deref().clone(),
            &asset_server,
        ));
        if let Some(task) = grpc_client.load_game(join.id) {
            commands.spawn(CallGetGame(task));
        } else {
            println!("load_game failed");
        }
        println!("state transition: join game");
        next_app_state.set(AppState::Game);
        commands.play_sound(&asset_server, CONFIRMATION_SOUND_PATH);
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
                    commands.play_sound(&asset_server, CONFIRMATION_SOUND_PATH);
                } else {
                    commands.play_sound(&asset_server, ERROR_SOUND_PATH);
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

    commands.spawn(overlay_ui_node()).with_children(|parent| {
        parent.spawn(column_node_bundle()).with_children(|parent| {
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
    });
}

fn exit_pause(
    mut commands: Commands,
    ui_nodes: Query<(Entity, Option<&Overlay>), With<bevy::ui::Node>>,
    state: Res<State<AppState>>,
) {
    if *state.get() == AppState::Game {
        if let Some((entity, _)) = ui_nodes.iter().find(|(_, overlay)| overlay.is_some()) {
            commands.entity(entity).despawn_recursive();
        }
    } else {
        for (entity, _) in ui_nodes.iter() {
            commands.entity(entity).despawn();
        }
    }
}

fn setup_game(
    mut commands: Commands,
    mut game: ResMut<CurrentGame>,
    mut state_updated: EventWriter<StateUpdated>,
    mut player_info_ready: EventWriter<PlayerInfoReady>,
) {
    commands
        .spawn(global_column_node_bundle())
        .with_children(|builder| {
            let player_id = game.user_id();
            let Some(enemy_id) = game.get_enemy() else {
                println!("unable to get enemy id, CurrentGame is corrupted");
                return;
            };
            builder.spawn(InGameUIBundle::new(player_id, enemy_id));
            if let Some(image) = game.get_player_image(&player_id) {
                player_info_ready.send(PlayerInfoReady::new(player_id, image.clone()));
            }
            if let Some(image) = game.get_player_image(&enemy_id) {
                player_info_ready.send(PlayerInfoReady::new(enemy_id, image.clone()));
            }
            let board = builder.spawn(board::BoardBundle::default()).id();
            game.set_board(board);
            state_updated.send(StateUpdated(game.state()));
        });
}

fn make_turn(
    mut commands: Commands,
    mut board_button_pressed: EventReader<board::ButtonPressed>,
    game: Res<CurrentGame>,
    asset_server: Res<AssetServer>,
    client: ResMut<GrpcClient>,
) {
    for event in board_button_pressed.read() {
        if let Some(task) =
            client.make_turn(game.id(), game.user_id(), event.pos.row(), event.pos.col())
        {
            commands.spawn((CallMakeTurn(task), event.pos));
        } else {
            println!("grpc server is down");
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
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
    mut create_game: Query<(Entity, &mut CallCreateGame)>,
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, mut task) in create_game.iter_mut() {
        if let Some(res) = block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).despawn();
            match res {
                Ok(response) => {
                    let Some(game_info) = response.into_inner().game_info else {
                        println!("received empty CreateGame reply from server");
                        commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                        return;
                    };
                    let info = match GameInfo::try_from(game_info) {
                        Ok(info) => info,
                        Err(err) => {
                            println!("failed to decode game info: {}", err);
                            continue;
                        }
                    };
                    if let Some(&user_id) = info.players.first() {
                        println!("starting created game: {}", info.id);
                        commands.insert_resource(CurrentGame::new_from_asset_server(
                            user_id,
                            info,
                            &asset_server,
                        ));
                        next_app_state.set(AppState::Game);
                        commands.play_sound(&asset_server, CONFIRMATION_SOUND_PATH);
                    } else {
                        println!("GameInfo is corrupted: players is empty");
                        commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                    }
                }
                Err(err) => {
                    println!("CreateGame request failed: {}", err);
                    commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                }
            };
        }
    }
}

fn handle_cell_updated(
    mut cell_updated: EventReader<CellUpdated>,
    mut board_content: EventWriter<board::ButtonContentArrived>,
    game: Res<CurrentGame>,
) {
    for event in cell_updated.read() {
        let Some(img) = game.get_player_image(&event.player_id()) else {
            println!("failed to get player image, dropping event");
            continue;
        };
        let Some(board) = game.board_entity() else {
            println!("ui board is not set for this game, dropping event");
            continue;
        };
        board_content.send(board::ButtonContentArrived {
            board: *board,
            pos: event.pos(),
            image: img.clone(),
        });
    }
}

fn handle_successful_turn(
    mut commands: Commands,
    mut successful_turn: EventReader<SuccessfulTurn>,
    asset_server: Res<AssetServer>,
) {
    for _event in successful_turn.read() {
        commands.play_sound(&asset_server, TURN_SOUND_PATH);
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
        commands.spawn(overlay_ui_node()).with_children(|parent| {
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
