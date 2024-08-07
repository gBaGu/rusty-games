use std::ops::Deref;

use bevy::app::AppExit;
use bevy::asset::io::file::FileAssetReader;
use bevy::asset::AssetServer;
use bevy::ecs::change_detection::{Res, ResMut};
use bevy::ecs::entity::Entity;
use bevy::ecs::event::{EventReader, EventWriter};
use bevy::ecs::query::{Changed, With};
use bevy::ecs::schedule::{NextState, State};
use bevy::ecs::system::{Commands, Query};
use bevy::hierarchy::{BuildChildren, DespawnRecursiveExt};
use bevy::input::{keyboard::KeyCode, mouse::MouseButton, ButtonInput};
use bevy::math::{Vec2, Vec3};
use bevy::time::Time;
use bevy::ui::node_bundles::TextBundle;
use bevy::ui::widget::Button;
use bevy::ui::{Interaction, Style, UiRect, Val};
use bevy::utils::default;
use bevy::window::{PrimaryWindow, Window};
use bevy_simple_text_input::{TextInputInactive, TextInputSubmitEvent, TextInputValue};
use game_server::game::{FinishedState, Game, GameState};
use game_server::proto;
use tic_tac_toe_ai::Agent;

use super::components::{
    CreateGame, JoinGame, MenuNavigationButtonBundle, NetworkGameTextInputBundle, Overlay,
    OverlayNodeBundle, Setting, SettingTextInputBundle, SubmitButton, SubmitButtonBundle,
    UiImageBundle,
};
use super::game_list::{GameList, GameListBundle};
use super::ingame::InGameUIBundle;
use super::resources::RefreshGamesTimer;
use crate::app_state::{AppState, AppStateTransition, MenuState};
use crate::board::{BoardBundle, TileFilled, TilePressed};
use crate::bot::{Bot, MoveStrategy, AGENT_PATH};
use crate::commands::{CommandsExt, EntityCommandsExt};
use crate::game::{
    Authority, CellUpdated, CurrentGame, GameExit, GameInfo, LocalGame, LocalGameTurn,
    NetworkGameTurn, StateUpdated, SuccessfulTurn,
};
use crate::grpc::{GrpcClient, RpcResultReady};
use crate::interface::common::{
    column_node_bundle, menu_item_style, menu_text_style, root_node_bundle, row_node_bundle,
    CONFIRMATION_SOUND_PATH, ERROR_SOUND_PATH, LOGO_HEIGHT, LOGO_WIDTH, TURN_SOUND_PATH,
};
use crate::interface::events::{PlayerGamesReady, SubmitPressed};
use crate::Settings;

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
                        let local_game = LocalGame::default();
                        let state = local_game.state();
                        commands.insert_resource(local_game);
                        let bot_id = 0;
                        let game = CurrentGame::new_with_bot(
                            user_id,
                            bot_id,
                            true,
                            state,
                            Default::default(),
                            &asset_server,
                        );
                        commands.insert_resource(game);
                        if let Ok(agent) =
                            Agent::load(FileAssetReader::get_base_path().join(AGENT_PATH))
                        {
                            commands.spawn(Bot::new(bot_id, MoveStrategy::QLearningModel(agent)));
                        } else {
                            println!("failed to load agent");
                        }
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
    let Some(user_id) = settings.user_id() else {
        return;
    };
    if let Some((_, join)) = join_button.iter().find(|(&i, _)| i == Interaction::Pressed) {
        let game = match CurrentGame::new_over_network(user_id, join.deref().clone(), &asset_server)
        {
            Ok(game) => game,
            Err(err) => {
                println!("unable to join game: {}", err);
                commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                return;
            }
        };
        commands.insert_resource(game);
        match grpc_client.get_game(proto::GameType::TicTacToe, join.id) {
            Ok(task) => {
                commands.spawn(task);
            }
            Err(err) => println!("get_game call failed: {}", err),
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
                match grpc_client.create_game(proto::GameType::TicTacToe, user_id, opponent_id) {
                    Ok(task) => {
                        commands.spawn(task);
                        return;
                    }
                    Err(err) => println!("create_game call failed: {}", err),
                }
            }
        }
        commands.play_sound(&asset_server, ERROR_SOUND_PATH);
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

pub fn submit_setting(
    text_input: Query<(Entity, &TextInputValue, &Setting)>,
    mut commands: Commands,
    mut submit_pressed: EventReader<SubmitPressed>,
    mut text_input_submit: EventReader<TextInputSubmitEvent>,
    mut settings: ResMut<Settings>,
    asset_server: Res<AssetServer>,
) {
    let mut submit = |input: &str, setting: &Setting| match setting {
        Setting::UserId => {
            if let Ok(val) = input.parse::<u64>() {
                settings.set_user_id(val);
                commands.play_sound(&asset_server, CONFIRMATION_SOUND_PATH);
            } else {
                commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            }
        }
    };
    for event in submit_pressed.read() {
        let Some((_, input, setting)) = text_input.iter().find(|(e, _, _)| *e == event.source)
        else {
            continue;
        };
        submit(&input.0, setting);
    }
    for event in text_input_submit.read() {
        let Some((_, _, setting)) = text_input.iter().find(|(e, _, _)| *e == event.entity) else {
            continue;
        };
        submit(&event.value, setting);
    }
}

pub fn cleanup_ui(mut commands: Commands, ui_nodes: Query<Entity, With<bevy::ui::Node>>) {
    for entity in ui_nodes.iter() {
        commands.entity(entity).despawn();
    }
}

pub fn despawn_board(mut commands: Commands, game: Res<CurrentGame>) {
    if let Some(entity) = game.board_entity() {
        commands.entity(*entity).despawn_recursive();
    }
}

pub fn setup_main_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let logo = asset_server.load("sprites/logo.png");
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

    commands.spawn(root_node_bundle()).with_children(|builder| {
        builder.spawn(UiImageBundle::new(
            Style {
                height: Val::Px(LOGO_HEIGHT),
                width: Val::Px(LOGO_WIDTH),
                margin: UiRect::all(Val::Px(20.)),
                ..default()
            },
            logo,
        ));
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

    commands.spawn(root_node_bundle()).with_children(|builder| {
        builder.spawn(row_node_bundle()).with_children(|builder| {
            builder.spawn(TextBundle::from_section("Set user id:", text_style.clone()));
            let input = builder
                .spawn(SettingTextInputBundle::new(
                    style.clone(),
                    text_style.clone(),
                    Setting::UserId,
                ))
                .id();
            builder
                .spawn(SubmitButtonBundle::new(style.clone(), input))
                .with_child(TextBundle::from_section("Save", text_style.clone()));
        });
        builder
            .spawn(MenuNavigationButtonBundle::new(
                style,
                AppState::Menu(MenuState::Main),
            ))
            .with_child(TextBundle::from_section("Back", text_style));
    });
}

pub fn setup_play_with_bot_menu(mut commands: Commands, asset_server: Res<AssetServer>) {
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

    commands.spawn(root_node_bundle()).with_children(|builder| {
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
    grpc_client: Option<Res<GrpcClient>>,
) {
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

    let mut game_list = GameListBundle::default();
    if let Some(id) = settings.user_id() {
        match grpc_client {
            Some(client) if client.connected() => {
                match client.get_player_games(proto::GameType::TicTacToe, id) {
                    Ok(task) => {
                        commands.spawn(task);
                    }
                    Err(err) => {
                        println!("get_player_games call failed: {}", err);
                        game_list.list = GameList::Message("Server is down".into());
                    }
                }
            }
            _ => game_list.list = GameList::Message("Server is down".into()),
        };
    } else {
        game_list.list = GameList::Message("No user id provided".into());
    }
    commands.spawn(root_node_bundle()).with_children(|builder| {
        builder.spawn(row_node_bundle()).with_children(|builder| {
            builder.spawn(TextBundle::from_section("Opponent id:", text_style.clone()));
            let input = builder
                .spawn(NetworkGameTextInputBundle::new(
                    style.clone(),
                    text_style.clone(),
                ))
                .id();
            builder
                .spawn(SubmitButtonBundle::new(style.clone(), input))
                .with_child(TextBundle::from_section("Create game", text_style.clone()));
        });
        builder.spawn(game_list);
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
        .with_children(|builder| {
            builder
                .spawn(column_node_bundle())
                .with_children(|builder| {
                    for (state, text) in [
                        (AppState::Game, "Resume"),
                        (AppState::Menu(MenuState::Settings), "Settings"),
                        (AppState::Menu(MenuState::Main), "Main menu"),
                    ] {
                        builder
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
    window: Query<&Window, With<PrimaryWindow>>,
    mut game: ResMut<CurrentGame>,
    mut state_updated: EventWriter<StateUpdated>,
) {
    let Ok(window) = window.get_single() else {
        println!("failed to get single window");
        return;
    };
    let interface_height = window.height() * 0.2;
    let board_area_height = window.height() - interface_height;
    commands.spawn(root_node_bundle()).with_children(|builder| {
        let player = game.user_data();
        let enemy = game.enemy_data();
        builder.spawn(InGameUIBundle::new(
            player.auth(),
            player.game_player_id(),
            player.image().clone(),
            enemy.auth(),
            enemy.game_player_id(),
            enemy.image().clone(),
        ));
    });
    let board_size = window.width().min(board_area_height) * 0.8;
    let board_size = Vec2::new(board_size, board_size);
    let board_translation = Vec3::new(0.0, -interface_height / 2.0, 0.0);
    let board = commands
        .spawn(BoardBundle::new(board_size, board_translation))
        .id();
    game.set_board(board);
    state_updated.send(StateUpdated(game.state()));
}

/// Tracks ButtonPressed event from board and transforms it into NetworkGameTurn/LocalGameTurn event
pub fn make_turn(
    mut commands: Commands,
    mut board_button_pressed: EventReader<TilePressed>,
    mut network_turn_data: EventWriter<NetworkGameTurn>,
    mut local_turn_data: EventWriter<LocalGameTurn>,
    game: Res<CurrentGame>,
    settings: Res<Settings>,
    asset_server: Res<AssetServer>,
) {
    let Some(user_id) = settings.user_id() else {
        println!("user id is not set");
        return;
    };
    for event in board_button_pressed.read() {
        if game.board()[(event.pos().row() as usize, event.pos().col() as usize).into()].is_some() {
            println!("cell is occupied");
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            continue;
        }
        if matches!(game.state(), GameState::Finished(_)) {
            println!("cannot make turn in finished game");
            commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            continue;
        }
        game.trigger_turn(
            &mut network_turn_data,
            &mut local_turn_data,
            Authority::Player(user_id),
            event.pos(),
        );
    }
}

pub fn send_get_player_games(
    mut game_list: Query<&mut GameList>,
    mut commands: Commands,
    mut timer: ResMut<RefreshGamesTimer>,
    client: Res<GrpcClient>,
    settings: Res<Settings>,
    time: Res<Time>,
) {
    if timer.tick(time.delta()).just_finished() {
        let Ok(mut list) = game_list.get_single_mut() else {
            println!("no game list found to refresh");
            return;
        };
        let Some(id) = settings.user_id() else {
            *list = GameList::Message("No user id provided".into());
            return;
        };
        match client.get_player_games(proto::GameType::TicTacToe, id) {
            Ok(task) => {
                commands.spawn(task);
                timer.reset();
                timer.pause();
            }
            Err(err) => {
                println!("get_player_games call failed: {}", err);
                *list = GameList::Message("Server is down".into());
            }
        }
    }
}

pub fn handle_get_player_games(
    mut get_games_result: EventReader<RpcResultReady<proto::GetPlayerGamesReply>>,
    mut timer: ResMut<RefreshGamesTimer>,
    mut games_ready: EventWriter<PlayerGamesReady>,
) {
    for event in get_games_result.read() {
        timer.unpause();
        match event.deref() {
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

pub fn handle_create_game_task(
    mut create_game_result: EventReader<RpcResultReady<proto::CreateGameReply>>,
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    settings: Res<Settings>,
    asset_server: Res<AssetServer>,
) {
    let Some(user_id) = settings.user_id() else {
        return;
    };
    for event in create_game_result.read() {
        match event.deref() {
            Ok(response) => {
                let Some(game_info) = &response.get_ref().game_info else {
                    println!("received empty CreateGame reply from server");
                    commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                    return;
                };
                let info = match GameInfo::try_from(game_info.clone()) {
                    Ok(info) => info,
                    Err(err) => {
                        println!("failed to decode game info: {}", err);
                        commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                        continue;
                    }
                };
                println!("starting created game: {}", info.id);
                let game = match CurrentGame::new_over_network(user_id, info, &asset_server) {
                    Ok(game) => game,
                    Err(err) => {
                        println!("unable to join game: {}", err);
                        commands.play_sound(&asset_server, ERROR_SOUND_PATH);
                        return;
                    }
                };
                commands.insert_resource(game);
                next_app_state.set(AppState::Game);
                commands.play_sound(&asset_server, CONFIRMATION_SOUND_PATH);
            }
            Err(err) => {
                println!("CreateGame request failed: {}", err);
                commands.play_sound(&asset_server, ERROR_SOUND_PATH);
            }
        };
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

pub fn handle_cell_updated(
    mut cell_updated: EventReader<CellUpdated>,
    mut tile_filled: EventWriter<TileFilled>,
    game: Res<CurrentGame>,
) {
    for event in cell_updated.read() {
        let Some(img) = game.get_player_image(event.player_id()) else {
            println!("failed to get player image, dropping event");
            continue;
        };
        let Some(board) = game.board_entity() else {
            println!("ui board is not set for this game, dropping event");
            continue;
        };
        tile_filled.send(TileFilled::new(*board, event.pos(), img.clone()));
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

pub fn create_game_over_overlay(
    mut commands: Commands,
    mut game_exit: EventReader<GameExit>,
    game: Res<CurrentGame>,
    asset_server: Res<AssetServer>,
) {
    if game_exit.read().next().is_some() {
        let GameState::Finished(state) = game.state() else {
            return;
        };
        let text_style = menu_text_style(&asset_server);
        let style = menu_item_style();
        let text = match state {
            FinishedState::Win(id) if id == game.user_data().game_player_id() => "You win!",
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
