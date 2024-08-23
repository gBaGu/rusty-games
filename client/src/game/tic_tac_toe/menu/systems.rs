use bevy::prelude::*;
use bevy::utils::petgraph::matrix_graph::Nullable;
use bevy_simple_text_input::{TextInputSubmitEvent, TextInputValue};
use game_server::proto;

use crate::commands::EntityCommandsExt;
use crate::game::components::BotDifficultyButtonBundle;
use crate::game::tic_tac_toe::bot::Strategy;
use crate::game::tic_tac_toe::components::{CreateGameContext, EnemyType};
use crate::game::tic_tac_toe::menu::components::{
    BotGameSettings, BotGameSettingsBundle, BotStrategyButtonBundle, CommonGameSettings,
    NetworkGameSettings, NetworkGameSettingsBundle,
};
use crate::game::{BotDifficulty, CreateGame, NetworkGame, PendingExistingGameBundle, PendingNewGameBundle};
use crate::grpc::GrpcClient;
use crate::interface;
use crate::interface::{GameReady, JoinPressed};
use crate::Settings;

pub fn init_bot_settings_menu(
    mut commands: Commands,
    node: Query<Entity, Added<interface::GameSettings>>,
    asset_server: Res<AssetServer>,
) {
    for settings_entity in node.iter() {
        let text_style = interface::common::menu_text_style(&asset_server);
        let mut choosable_setting_style = interface::common::menu_item_style();
        choosable_setting_style.border = UiRect::all(Val::Px(2.0));
        commands
            .entity(settings_entity)
            .insert(BotGameSettingsBundle::default())
            .with_children(|builder| {
                builder.spawn(TextBundle::from_section("Bot", text_style.clone()));
                builder
                    .spawn(interface::common::row_node_bundle())
                    .with_children(|builder| {
                        builder
                            .spawn(BotStrategyButtonBundle::new(
                                choosable_setting_style.clone(),
                                Strategy::Random,
                                settings_entity,
                            ))
                            .with_child(TextBundle::from_section("Random", text_style.clone()));
                        builder
                            .spawn(BotStrategyButtonBundle::new(
                                choosable_setting_style.clone(),
                                Strategy::QLearning,
                                settings_entity,
                            ))
                            .with_child(TextBundle::from_section("QLearning", text_style.clone()));
                    });
                builder
                    .spawn(interface::common::row_node_bundle())
                    .with_children(|builder| {
                        builder
                            .spawn(BotDifficultyButtonBundle::new(
                                choosable_setting_style.clone(),
                                BotDifficulty::Easy,
                                settings_entity,
                            ))
                            .with_child(TextBundle::from_section("Easy", text_style.clone()));
                        builder
                            .spawn(BotDifficultyButtonBundle::new(
                                choosable_setting_style.clone(),
                                BotDifficulty::Medium,
                                settings_entity,
                            ))
                            .with_child(TextBundle::from_section("Medium", text_style.clone()));
                        builder
                            .spawn(BotDifficultyButtonBundle::new(
                                choosable_setting_style.clone(),
                                BotDifficulty::Hard,
                                settings_entity,
                            ))
                            .with_child(TextBundle::from_section("Hard", text_style.clone()));
                    });
            });
    }
}

pub fn init_network_settings_menu(
    mut commands: Commands,
    node: Query<Entity, Added<interface::GameSettings>>,
    asset_server: Res<AssetServer>,
) {
    for settings_entity in node.iter() {
        let text_style = interface::common::menu_text_style(&asset_server);
        let style = interface::common::menu_item_style();
        commands
            .entity(settings_entity)
            .insert(NetworkGameSettingsBundle::default())
            .with_children(|builder| {
                builder
                    .spawn(interface::common::row_node_bundle())
                    .with_children(|builder| {
                        builder.spawn(TextBundle::from_section("Opponent id:", text_style.clone()));
                        let input_id = builder
                            .spawn(interface::UserIdTextInputBundle::new(
                                style.clone(),
                                text_style.clone(),
                            ))
                            .insert(interface::GameSettingsLink::new(settings_entity))
                            .id();
                        builder
                            .spawn(interface::SubmitButtonBundle::new(style, input_id))
                            .with_child(TextBundle::from_section("Save", text_style.clone()));
                    });
            });
    }
}

pub fn init_game_list(
    mut commands: Commands,
    mut game_list: Query<&mut interface::GameList, Added<interface::GameList>>,
    settings: Res<Settings>,
    client: Option<Res<GrpcClient>>,
) {
    for mut game_list in game_list.iter_mut() {
        if let Some(id) = settings.user_id() {
            match client.as_ref() {
                Some(client) if client.connected() => {
                    match client.get_player_games(proto::GameType::TicTacToe, id) {
                        Ok(task) => {
                            commands.spawn(task);
                        }
                        Err(err) => {
                            println!("get_player_games call failed: {}", err);
                            *game_list = interface::GameList::Message("Server is down".into());
                        }
                    }
                }
                _ => *game_list = interface::GameList::Message("Server is down".into()),
            };
        } else {
            *game_list = interface::GameList::Message("No user id provided".into());
        }
    }
}

pub fn send_get_player_games(
    mut game_list: Query<&mut interface::GameList>,
    mut commands: Commands,
    mut timer: ResMut<interface::RefreshGamesTimer>,
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
            *list = interface::GameList::Message("No user id provided".into());
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
                *list = interface::GameList::Message("Server is down".into());
            }
        }
    }
}

pub fn update_active_strategy(
    button: Query<(&Strategy, &interface::GameSettingsLink), With<Button>>,
    mut settings: Query<&mut BotGameSettings>,
    mut setting_pressed: EventReader<interface::SettingOptionPressed>,
) {
    for event in setting_pressed.read() {
        let Ok((strategy, settings_link)) = button.get(event.source) else {
            continue;
        };
        if settings_link.get() != event.settings {
            println!("settings link from event doesn't match one from component");
            continue;
        }
        let Ok(mut settings) = settings.get_mut(settings_link.get()) else {
            println!("unable to get strategy settings component");
            continue;
        };
        settings.set_strategy(*strategy);
    }
}

pub fn update_strategy_button_border(
    mut button: Query<(Entity, &mut BorderColor), (With<Button>, With<Strategy>)>,
    mut setting_pressed: EventReader<interface::SettingOptionPressed>,
) {
    for event in setting_pressed.read() {
        if !button.contains(event.source) {
            continue;
        }
        for (entity, mut border) in button.iter_mut() {
            if entity == event.source {
                *border = interface::common::SECONDARY_COLOR.into();
            } else {
                *border = Color::NONE.into();
            }
        }
    }
}

pub fn update_active_difficulty(
    button: Query<(&BotDifficulty, &interface::GameSettingsLink), With<Button>>,
    mut settings: Query<&mut BotGameSettings>,
    mut setting_pressed: EventReader<interface::SettingOptionPressed>,
) {
    for event in setting_pressed.read() {
        let Ok((difficulty, settings_link)) = button.get(event.source) else {
            continue;
        };
        if settings_link.get() != event.settings {
            println!("settings link from event doesn't match one from component");
            continue;
        }
        let Ok(mut settings) = settings.get_mut(settings_link.get()) else {
            println!("unable to get strategy settings component");
            continue;
        };
        settings.set_difficulty(*difficulty);
    }
}

pub fn create_bot_game_pressed(
    mut commands: Commands,
    button: Query<
        (&Interaction, &interface::GameSettingsLink),
        (Changed<Interaction>, With<interface::CreateGame>),
    >,
    game_settings: Query<(&CommonGameSettings, &BotGameSettings)>,
    mut create_game: EventWriter<CreateGame>,
    settings: Res<Settings>,
) {
    for (interaction, settings_link) in button.iter() {
        if *interaction == Interaction::Pressed {
            let Some(user) = settings.user_id() else {
                continue;
            };
            let Ok((common_settings, bot_settings)) = game_settings.get(settings_link.get()) else {
                continue;
            };
            let Some(strategy) = bot_settings.strategy() else {
                continue;
            };
            let mut difficulty = None;
            if strategy.has_difficulty() {
                if bot_settings.difficulty().is_none() {
                    println!("bot difficulty is not set");
                    continue;
                }
                difficulty = bot_settings.difficulty();
            }

            let ctx = CreateGameContext::new(
                EnemyType::Bot {
                    strategy,
                    difficulty,
                },
                common_settings.user_first(),
                None,
            );
            let context_entity = commands.spawn(ctx).id();
            create_game.send(CreateGame::new_local(user, context_entity));
        }
    }
}

pub fn create_network_game_pressed(
    mut commands: Commands,
    button: Query<
        (&Interaction, &interface::GameSettingsLink),
        (Changed<Interaction>, With<interface::CreateGame>),
    >,
    game_settings: Query<(&CommonGameSettings, &NetworkGameSettings)>,
    client: Option<Res<GrpcClient>>,
    settings: Res<Settings>,
) {
    for (interaction, settings_link) in button.iter() {
        if *interaction == Interaction::Pressed {
            let Some(user) = settings.user_id() else {
                continue;
            };
            let Ok((_, network_settings)) = game_settings.get(settings_link.get()) else {
                continue;
            };
            let Some(opponent) = network_settings.opponent() else {
                println!("opponent is not set");
                continue;
            };
            let Some(client) = client.as_ref() else {
                println!("grpc client is not connected");
                continue;
            };
            match client.create_game(proto::GameType::TicTacToe, user, opponent) {
                Ok(task) => {
                    commands.spawn(task);
                    commands.spawn(PendingNewGameBundle::new());
                    return;
                }
                Err(err) => println!("create_game call failed: {}", err),
            }
        }
    }
}

pub fn save_opponent_pressed(
    mut settings: Query<&mut NetworkGameSettings>,
    input: Query<(&TextInputValue, &interface::GameSettingsLink), With<interface::UserIdInput>>,
    mut submit_pressed: EventReader<interface::SubmitPressed>,
    mut text_input_submit: EventReader<TextInputSubmitEvent>,
) {
    let mut submit = |input: &str, settings: &mut NetworkGameSettings| {
        if let Ok(val) = input.parse::<u64>() {
            settings.set_opponent(val);
        }
    };
    for event in submit_pressed.read() {
        let Ok((input_value, settings_link)) = input.get(event.source) else {
            continue;
        };
        let Ok(mut settings) = settings.get_mut(settings_link.get()) else {
            continue;
        };
        submit(&input_value.0, &mut settings);
    }
    for event in text_input_submit.read() {
        let Ok((_, settings_link)) = input.get(event.entity) else {
            continue;
        };
        let Ok(mut settings) = settings.get_mut(settings_link.get()) else {
            continue;
        };
        submit(&event.value, &mut settings);
    }
}

pub fn join_pressed(
    mut commands: Commands,
    game: Query<(Entity, &NetworkGame)>,
    mut join_pressed: EventReader<JoinPressed>,
    mut game_ready: EventWriter<GameReady>,
    client: Option<Res<GrpcClient>>,
) {
    for event in join_pressed.read() {
        if let Some((game_entity, _)) = game.iter().find(|(_, &game)| *game == event.game_id()) {
            game_ready.send(GameReady::new(game_entity));
            return;
        }
        if let Some(client) = client.as_ref() {
            match client.get_game(proto::GameType::TicTacToe, event.game_id()) {
                Ok(task) => {
                    commands.spawn(task);
                    commands.spawn(PendingExistingGameBundle::new(event.game_id()));
                }
                Err(err) => println!("get_game call failed: {}", err),
            };
        }
    }
}
