use std::ops::Deref;

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy_simple_text_input::{TextInputInactive, TextInputSubmitEvent, TextInputValue};
use game_server::proto;

use super::components::{
    CreateGame, JoinGame, MenuNavigationButtonBundle, NetworkGameTextInputBundle, Overlay,
    OverlayNodeBundle, Playground, PlaygroundBundle, Setting, SettingTextInputBundle, SubmitButton,
    SubmitButtonBundle, UiImageBundle,
};
use super::game_list::{GameList, GameListBundle};
use super::resources::RefreshGamesTimer;
use super::{GameReady, GameReadyToExit};
use crate::app_state::{AppState, AppStateTransition, MenuState};
use crate::commands::{CommandsExt, EntityCommandsExt};
use crate::game::{
    ActiveGame, Board, BotDifficulty, BotStrategy, CreateTicTacToeGame, CreateTicTacToeGameContext,
    CurrentUser, EnemyType, GameInfo, GameLink, NetworkGame, PendingExistingGameBundle,
    PendingNewGameBundle, TTTBotQLearningStrategy, TTTBotStrategy, Winner,
};
use crate::grpc::{GrpcClient, RpcResultReady};
use crate::interface::common::{
    column_node_bundle, menu_item_style, menu_text_style, root_node_bundle, row_node_bundle,
    CONFIRMATION_SOUND_PATH, ERROR_SOUND_PATH, LOGO_HEIGHT, LOGO_WIDTH,
};
use crate::interface::events::{GameExit, PlayerGamesReady, SubmitPressed};
use crate::Settings;

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
    mut commands: Commands,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut create_game: EventWriter<CreateTicTacToeGame>,
    mut exit: EventWriter<AppExit>,
    app_state: Res<State<AppState>>,
    settings: Res<Settings>,
    asset_server: Res<AssetServer>,
) {
    for (interaction, state_transition) in menu_items.iter() {
        if *interaction == Interaction::Pressed {
            if let Some(new_state) = state_transition.0 {
                if *app_state.get() == AppState::Menu(MenuState::PlayWithBot)
                    && new_state == AppState::Game
                {
                    if let Some(user) = settings.user_id() {
                        create_game.send(CreateTicTacToeGame::new_local(
                            CreateTicTacToeGameContext::new(
                                user,
                                EnemyType::Bot(BotStrategy::new(TTTBotStrategy::QLearning(
                                    TTTBotQLearningStrategy::new(BotDifficulty::Hard),
                                ))),
                                true,
                                None,
                            ),
                        ));
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
    mut commands: Commands,
    game: Query<(Entity, &NetworkGame)>,
    join_button: Query<(&Interaction, &JoinGame), (With<Button>, Changed<Interaction>)>,
    mut game_ready: EventWriter<GameReady>,
    client: Res<GrpcClient>,
) {
    let Some((_, join)) = join_button.iter().find(|(&i, _)| i == Interaction::Pressed) else {
        return;
    };
    if let Some((game_entity, _)) = game.iter().find(|(_, &game)| *game == join.id) {
        game_ready.send(GameReady::new(game_entity));
        return;
    }
    match client.get_game(proto::GameType::TicTacToe, join.id) {
        Ok(task) => {
            commands.spawn(task);
            commands.spawn(PendingExistingGameBundle::new(join.id));
        }
        Err(err) => println!("get_game call failed: {}", err),
    };
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
    client: Res<GrpcClient>,
    asset_server: Res<AssetServer>,
) {
    let mut create = |input: &str| {
        if let Ok(opponent_id) = input.parse::<u64>() {
            if let Some(user_id) = settings.user_id() {
                match client.create_game(proto::GameType::TicTacToe, user_id, opponent_id) {
                    Ok(task) => {
                        commands.spawn(task);
                        commands.spawn(PendingNewGameBundle::new());
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

pub fn cleanup_ui(mut commands: Commands, ui_nodes: Query<Entity, With<Node>>) {
    for entity in ui_nodes.iter() {
        commands.entity(entity).despawn();
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
    client: Option<Res<GrpcClient>>,
) {
    let text_style = menu_text_style(&asset_server);
    let style = menu_item_style();

    let mut game_list = GameListBundle::default();
    if let Some(id) = settings.user_id() {
        match client {
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

pub fn clear_pause_overlay(mut commands: Commands, overlay: Query<Entity, With<Overlay>>) {
    for overlay in overlay.iter() {
        commands.entity(overlay).despawn_recursive();
    }
}

pub fn setup_game(mut commands: Commands, game: Query<Entity, With<ActiveGame>>) {
    if let Ok(game_entity) = game.get_single() {
        commands.spawn(PlaygroundBundle {
            node: root_node_bundle(),
            game_link: GameLink::new(game_entity),
            playground: Playground,
        });
    }
}

pub fn start_game(
    mut commands: Commands,
    mut game_ready: EventReader<GameReady>,
    mut next_app_state: ResMut<NextState<AppState>>,
    asset_server: Res<AssetServer>,
) {
    for event in game_ready.read() {
        println!("starting game: {:?}", event.get());
        next_app_state.set(AppState::Game);
        commands.entity(event.get()).insert(ActiveGame);
        commands.play_sound(&asset_server, CONFIRMATION_SOUND_PATH);
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
        let text_style = menu_text_style(&asset_server);
        let style = menu_item_style();
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

pub fn exit_game(
    mut commands: Commands,
    playground: Query<(Entity, &GameLink), With<Playground>>,
    board: Query<Entity, With<Board>>,
    mut game_exit: EventWriter<GameExit>,
) {
    for (playground, game_link) in playground.iter() {
        commands.entity(playground).despawn_recursive();
        game_exit.send(GameExit::new(game_link.get()));
    }
    for board in board.iter() {
        commands.entity(board).despawn_recursive();
    }
}

pub fn handle_game_exit(mut commands: Commands, mut game_exit: EventReader<GameExit>) {
    for event in game_exit.read() {
        commands.entity(event.get()).remove::<ActiveGame>();
    }
}
