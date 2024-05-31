use std::ops::Deref;
use std::str::FromStr;

use bevy::app::AppExit;
use bevy::asset::AssetServer;
use bevy::ecs::change_detection::{Res, ResMut};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{EventReader, EventWriter};
use bevy::ecs::query::{Changed, With, Without};
use bevy::ecs::schedule::{NextState, State};
use bevy::ecs::system::{Commands, Query};
use bevy::hierarchy::{BuildChildren, DespawnRecursiveExt};
use bevy::input::{keyboard::KeyCode, mouse::MouseButton, ButtonInput};
use bevy::prelude::TextBundle;
use bevy::tasks::{block_on, futures_lite::future};
use bevy::time::Time;
use bevy::ui::widget::Button;
use bevy::ui::Interaction;
use bevy::utils::default;
use bevy_simple_text_input::{TextInputInactive, TextInputSubmitEvent, TextInputValue};
use game_server::game::game::{FinishedState, GameState};

use super::components::{
    CreateGame, JoinGame, JoinGameButtonBundle, MenuNavigationButtonBundle,
    NetworkGameTextInputBundle, Overlay, OverlayNodeBundle, SubmitButton, SubmitButtonBundle,
    TextInputNodeBundle,
};
use super::game_list::{GameList, GameListBundle, LoadingGameListBundle};
use super::ingame::{InGameUIBundle, PlayerInfoReady};
use super::resources::RefreshGamesTimer;
use crate::app_state::{AppState, AppStateTransition, MenuState};
use crate::board;
use crate::commands::{CommandsExt, EntityCommandsExt};
use crate::game::{CellUpdated, CurrentGame, GameInfo, GameOver, StateUpdated, SuccessfulTurn};
use crate::grpc::{CallCreateGame, CallGetGame, CallGetPlayerGames, CallMakeTurn, GrpcClient};
use crate::interface::common::{
    column_node_bundle, global_column_node_bundle, menu_item_style, menu_text_style,
    row_node_bundle, CONFIRMATION_SOUND_PATH, ERROR_SOUND_PATH, TURN_SOUND_PATH,
};
use crate::interface::events::SubmitPressed;
use crate::settings::{submit_text_input_setting, Settings, SubmitTextInputSetting};

pub fn state_transition(
    menu_items: Query<(&Interaction, &AppStateTransition), (With<Button>, Changed<Interaction>)>,
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut exit: EventWriter<AppExit>,
    app_state: Res<State<AppState>>,
    settings: Res<Settings>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
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
    for (interaction, state_transition) in menu_items.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(new_state) = state_transition.0 {
                if *app_state.get() == AppState::Menu(MenuState::PlayWithBot)
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

pub fn join_game(
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

pub fn submit_press(
    submit_button: Query<(&Interaction, &SubmitButton), (With<Button>, Changed<Interaction>)>,
    mut submit_pressed: EventWriter<SubmitPressed>,
) {
    for (interaction, submit) in submit_button.iter() {
        if *interaction == Interaction::Pressed {
            submit_pressed.send(SubmitPressed {
                source: submit.source,
            });
        }
    }
}

pub fn create_game(
    text_input: Query<(Entity, &TextInputValue), With<CreateGame>>,
    mut commands: Commands,
    mut submit_pressed: EventReader<SubmitPressed>,
    mut text_input_submit: EventReader<TextInputSubmitEvent>,
    settings: Res<Settings>,
    grpc_client: Res<GrpcClient>,
    asset_server: Res<AssetServer>,
) {
    let mut create = |input: &str| {
        if let Ok(opponent_id) = input.parse::<u64>() {
            if let Some(user_id) = settings.user_id() {
                if let Some(task) = grpc_client.create_game(user_id, opponent_id) {
                    commands.spawn(CallCreateGame(task));
                    return;
                }
            }
        } else {
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
        }
    };
    for event in submit_pressed.read() {
        let Some((_, input)) = text_input.iter().find(|(e, _)| *e == event.source) else {
            continue;
        };
        create(&input.0);
    }
    for event in text_input_submit.read() {
        if !text_input.contains(event.entity) {
            continue;
        };
        create(&event.value);
    }
}

pub fn settings_submit<T: FromStr + 'static>(
    submit_button: Query<
        (&Interaction, &SubmitTextInputSetting<T>),
        (With<Button>, Changed<Interaction>),
    >,
    text_input: Query<(Entity, &TextInputValue)>,
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    asset_server: Res<AssetServer>,
) {
    for (interaction, submit_input) in submit_button.iter() {
        if *interaction == Interaction::Pressed {
            if let Some((_, val)) = text_input
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

pub fn cleanup_ui(mut commands: Commands, ui_nodes: Query<Entity, With<bevy::ui::Node>>) {
    for entity in ui_nodes.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn setup_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

    commands
        .spawn(global_column_node_bundle())
        .with_children(|builder| {
            for (state, text) in [
                (AppState::Menu(MenuState::PlayWithBot), "Play"),
                (AppState::Menu(MenuState::PlayOverNetwork), "Network"),
                (AppState::Menu(MenuState::Settings), "Settings"),
            ] {
                builder
                    .spawn(MenuNavigationButtonBundle::new(style.clone(), state))
                    .with_child(TextBundle::from_section(text, text_style.clone()));
            }
            builder
                .spawn(MenuNavigationButtonBundle::exit(style))
                .with_child(TextBundle::from_section("Exit", text_style));
        });
}

pub fn setup_settings_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

    commands
        .spawn(global_column_node_bundle())
        .with_children(|builder| {
            builder.spawn(row_node_bundle()).with_children(|builder| {
                builder.spawn(TextBundle::from_section("Set user id:", text_style.clone()));
                let input = builder
                    .spawn(TextInputNodeBundle::new(style.clone(), text_style.clone()))
                    .id();
                builder
                    .spawn(submit_text_input_setting(
                        style.clone(),
                        input,
                        Settings::set_user_id,
                    ))
                    .with_child(TextBundle::from_section("Save", text_style.clone()));
            });
            builder
                .spawn(MenuNavigationButtonBundle::new(
                    style.clone(),
                    AppState::Menu(MenuState::Main),
                ))
                .with_child(TextBundle::from_section("Back", text_style));
        });
}

pub fn setup_play_with_bot_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

    commands
        .spawn(global_column_node_bundle())
        .with_children(|builder| {
            builder
                .spawn(MenuNavigationButtonBundle::new(
                    style.clone(),
                    AppState::Game,
                ))
                .with_child(TextBundle::from_section(
                    "Play with bot",
                    text_style.clone(),
                ));
            builder
                .spawn(MenuNavigationButtonBundle::new(
                    style.clone(),
                    AppState::Menu(MenuState::Main),
                ))
                .with_child(TextBundle::from_section("Back", text_style));
        });
}

pub fn setup_play_over_network_menu(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
    grpc_client: Res<GrpcClient>,
) {
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

    commands
        .spawn(global_column_node_bundle())
        .with_children(|builder| {
            builder.spawn(row_node_bundle()).with_children(|builder| {
                builder.spawn(TextBundle::from_section("Opponent id:", text_style.clone()));
                let input = builder
                    .spawn(NetworkGameTextInputBundle::new(
                        text_style.clone(),
                        style.clone(),
                    ))
                    .id();
                builder
                    .spawn(SubmitButtonBundle::new(style.clone(), input))
                    .with_child(TextBundle::from_section("Create game", text_style.clone()));
            });
            if let Some(id) = settings.user_id() {
                if let Some(task) = grpc_client.load_player_games(id) {
                    builder.spawn(LoadingGameListBundle {
                        container: column_node_bundle(),
                        games: default(),
                        task: CallGetPlayerGames(task),
                    });
                } else {
                    builder
                        .spawn(GameListBundle {
                            container: column_node_bundle(),
                            games: default(),
                        })
                        .with_child(TextBundle::from_section(
                            "Server is down",
                            text_style.clone(),
                        ));
                }
            } else {
                builder
                    .spawn(GameListBundle {
                        container: column_node_bundle(),
                        games: default(),
                    })
                    .with_child(TextBundle::from_section(
                        "No user id provided",
                        text_style.clone(),
                    ));
            }
            builder
                .spawn(MenuNavigationButtonBundle::new(
                    style.clone(),
                    AppState::Menu(MenuState::Main),
                ))
                .with_child(TextBundle::from_section("Back", text_style));
        });
}

pub fn setup_pause(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

    commands
        .spawn(OverlayNodeBundle::default())
        .with_children(|parent| {
            parent.spawn(column_node_bundle()).with_children(|parent| {
                for (state, text) in [
                    (AppState::Game, "Resume"),
                    (AppState::Menu(MenuState::Settings), "Settings"),
                    (AppState::Menu(MenuState::Main), "Main menu"),
                ] {
                    parent
                        .spawn(MenuNavigationButtonBundle::new(style.clone(), state))
                        .with_child(TextBundle::from_section(text, text_style.clone()));
                }
            });
        });
}

pub fn exit_pause(
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

pub fn setup_game(
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

pub fn make_turn(
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

pub fn refresh_game_list(
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
                    commands.entity(entity).despawn_descendants().with_child(
                        TextBundle::from_section("Server is down", text_style.clone()),
                    );
                }
            } else {
                commands
                    .entity(entity)
                    .despawn_descendants()
                    .with_child(TextBundle::from_section(
                        "No user id provided",
                        text_style.clone(),
                    ));
            }
        }
    }
}

pub fn handle_player_games_task(
    mut game_lists: Query<(Entity, &mut GameList, &mut CallGetPlayerGames)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

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
                                        parent.spawn(TextBundle::from_section(
                                            "No games available",
                                            text_style.clone(),
                                        ));
                                        return;
                                    }
                                    for game in games.iter() {
                                        parent.spawn(row_node_bundle()).with_children(|parent| {
                                            parent.spawn(TextBundle::from_section(
                                                &format!("ID: {}", game.id),
                                                text_style.clone(),
                                            ));
                                            parent.spawn(TextBundle::from_section(
                                                &format!("{:?}", game.state),
                                                text_style.clone(),
                                            ));
                                            parent.spawn(TextBundle::from_section(
                                                &format!("{:?}", game.players),
                                                text_style.clone(),
                                            ));
                                            parent
                                                .spawn(JoinGameButtonBundle::new(
                                                    style.clone(),
                                                    game.clone(),
                                                ))
                                                .with_child(TextBundle::from_section(
                                                    "Join",
                                                    text_style.clone(),
                                                ));
                                        });
                                    }
                                });
                        }
                        Err(err) => {
                            println!("GetPlayerGames invalid response: {}", err);
                            commands.entity(entity).despawn_descendants().with_child(
                                TextBundle::from_section(
                                    &format!("Server error: {}", err),
                                    text_style.clone(),
                                ),
                            );
                        }
                    }
                }
                Err(err) => {
                    commands.entity(entity).despawn_descendants().with_child(
                        TextBundle::from_section(
                            &format!("Server error: {}", err.code().description()),
                            text_style.clone(),
                        ),
                    );
                }
            }
        }
    }
}

pub fn handle_create_game_task(
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

pub fn handle_cell_updated(
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

pub fn handle_successful_turn(
    mut commands: Commands,
    mut successful_turn: EventReader<SuccessfulTurn>,
    asset_server: Res<AssetServer>,
) {
    for _event in successful_turn.read() {
        commands.play_sound(&asset_server, TURN_SOUND_PATH);
    }
}

pub fn handle_game_over(
    mut commands: Commands,
    mut game_over: EventReader<GameOver>,
    game: Res<CurrentGame>,
    asset_server: Res<AssetServer>,
) {
    if let Some(event) = game_over.read().last() {
        let text_style = menu_text_style(&asset_server);
        let style = menu_item_style();
        let text = match event.deref() {
            FinishedState::Win(id) if *id == game.user_id() => "You win!",
            FinishedState::Win(_) => "You lose!",
            FinishedState::Draw => "It's a draw!",
        };
        commands
            .spawn(OverlayNodeBundle::default())
            .with_children(|builder| {
                builder
                    .spawn(column_node_bundle())
                    .with_children(|builder| {
                        builder.spawn(TextBundle::from_section(
                            &format!("Game over. {}", text),
                            text_style.clone(),
                        ));
                        builder
                            .spawn(MenuNavigationButtonBundle::new(
                                style.clone(),
                                AppState::Menu(MenuState::Main),
                            ))
                            .with_child(TextBundle::from_section("Main menu", text_style));
                    });
            });
    }
}
