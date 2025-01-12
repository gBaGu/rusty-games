use bevy::prelude::*;
use bevy_simple_text_input::{TextInputSubmitEvent, TextInputValue};
use game_server::core::tic_tac_toe::TicTacToe;

use crate::game::components::BotDifficultyButtonBundle;
use crate::game::tic_tac_toe::bot::Strategy;
use crate::game::tic_tac_toe::components::{CreateGameContext, EnemyType};
use crate::game::tic_tac_toe::menu::components::{
    BotGameSettings, BotGameSettingsBundle, BotStrategyButtonBundle, CommonGameSettings,
    NetworkGameSettings, NetworkGameSettingsBundle,
};
use crate::game::{
    BotDifficulty, GameDataReady, GameEntityReady, NetworkGame, PendingExistingGameBundle,
    PendingNewGameBundle,
};
use crate::Settings;
use crate::{grpc, interface};

/// Whenever [`interface::GameSettings`] is added to a page layout fill it
/// with bot game settings interface.
/// Should be called only from state `AppState::Menu(MenuState::PlayAgainstBot)`
pub fn init_bot_settings_menu(
    mut commands: Commands,
    node: Query<Entity, Added<interface::GameSettings>>,
    asset_server: Res<AssetServer>,
) {
    for settings_entity in node.iter() {
        let text_font = interface::common::load_text_font(&asset_server);
        let mut choosable_setting_node = interface::common::menu_item_node();
        choosable_setting_node.border = UiRect::all(Val::Px(2.0));
        commands
            .entity(settings_entity)
            .insert(BotGameSettingsBundle::default())
            .with_children(|builder| {
                builder.spawn(interface::TextBundle::new("Bot", text_font.clone()));
                builder
                    .spawn(interface::common::row_node())
                    .with_children(|builder| {
                        builder
                            .spawn(BotStrategyButtonBundle::new(
                                choosable_setting_node.clone(),
                                Strategy::Random,
                                settings_entity,
                            ))
                            .with_child(interface::TextBundle::new("Random", text_font.clone()));
                        builder
                            .spawn(BotStrategyButtonBundle::new(
                                choosable_setting_node.clone(),
                                Strategy::QLearning,
                                settings_entity,
                            ))
                            .with_child(interface::TextBundle::new("QLearning", text_font.clone()));
                    });
                builder
                    .spawn(interface::common::row_node())
                    .with_children(|builder| {
                        builder
                            .spawn(BotDifficultyButtonBundle::new(
                                choosable_setting_node.clone(),
                                BotDifficulty::Easy,
                                settings_entity,
                            ))
                            .with_child(interface::TextBundle::new("Easy", text_font.clone()));
                        builder
                            .spawn(BotDifficultyButtonBundle::new(
                                choosable_setting_node.clone(),
                                BotDifficulty::Medium,
                                settings_entity,
                            ))
                            .with_child(interface::TextBundle::new("Medium", text_font.clone()));
                        builder
                            .spawn(BotDifficultyButtonBundle::new(
                                choosable_setting_node.clone(),
                                BotDifficulty::Hard,
                                settings_entity,
                            ))
                            .with_child(interface::TextBundle::new("Hard", text_font.clone()));
                    });
            });
    }
}

/// Whenever [`interface::GameSettings`] is added to a page layout fill it
/// with network game settings interface.
/// Should be called only from state `AppState::Menu(MenuState::PlayOverNetwork)`
pub fn init_network_settings_menu(
    mut commands: Commands,
    node: Query<Entity, Added<interface::GameSettings>>,
    asset_server: Res<AssetServer>,
) {
    for settings_entity in node.iter() {
        let text_font = interface::common::load_text_font(&asset_server);
        let item_node = interface::common::menu_item_node();
        commands
            .entity(settings_entity)
            .insert(NetworkGameSettingsBundle::default())
            .with_children(|builder| {
                builder
                    .spawn(interface::common::row_node())
                    .with_children(|builder| {
                        builder.spawn(interface::TextBundle::new(
                            "Opponent id:",
                            text_font.clone(),
                        ));
                        let input_id = builder
                            .spawn(interface::UserIdTextInputBundle::new(
                                item_node.clone(),
                                text_font.clone(),
                            ))
                            .insert(interface::GameSettingsLink::new(settings_entity))
                            .id();
                        builder
                            .spawn(interface::SubmitButtonBundle::new(item_node, input_id))
                            .with_child(interface::TextBundle::new("Save", text_font.clone()));
                    });
            });
    }
}

/// Whenever [`interface::GameList`] is added to a page layout send GetPlayerGames request or
/// fill it with a status message.
pub fn init_game_list(
    mut commands: Commands,
    mut game_list: Query<(Entity, &mut interface::GameList), Added<interface::GameList>>,
    settings: Res<Settings>,
    client: Option<Res<grpc::GrpcClient>>,
) {
    for (game_list_entity, mut game_list) in game_list.iter_mut() {
        if let Some(id) = settings.user_id() {
            match client.as_ref() {
                Some(client) if client.connected() => {
                    match client.get_player_games::<TicTacToe>(id) {
                        Ok(task) => {
                            commands.entity(game_list_entity).insert(task);
                        }
                        Err(err) => {
                            println!("get_player_games call failed: {}", err);
                            *game_list = interface::GameList::Message("Server is down".into());
                        }
                    };
                }
                _ => *game_list = interface::GameList::Message("Server is down".into()),
            };
        } else {
            *game_list = interface::GameList::Message("No user id provided".into());
        }
    }
}

/// Whenever timer is finished send GetPlayerGames request or fill game list with a status message.
pub fn send_get_player_games(
    mut game_list: Query<(Entity, &mut interface::GameList)>,
    mut commands: Commands,
    mut timer: ResMut<interface::RefreshGamesTimer>,
    client: Res<grpc::GrpcClient>,
    settings: Res<Settings>,
    time: Res<Time>,
) {
    if timer.tick(time.delta()).just_finished() {
        let Ok((game_list_entity, mut list)) = game_list.get_single_mut() else {
            println!("no game list found to refresh");
            return;
        };
        let Some(id) = settings.user_id() else {
            *list = interface::GameList::Message("No user id provided".into());
            return;
        };
        match client.get_player_games::<TicTacToe>(id) {
            Ok(task) => {
                commands.entity(game_list_entity).insert(task);
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

/// Save chosen strategy into a [`BotGameSettings`] component.
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

/// Set border color for a strategy button that has just been pressed, reset it for
/// other strategy buttons.
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

/// Save chosen difficulty to a [`BotGameSettings`] component.
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

/// Get all data required to create a bot game, spawn [`CreateGameContext`] component and
/// send [`GameDataReady`] event.
pub fn create_bot_game(
    mut commands: Commands,
    button: Query<
        (&Interaction, &interface::GameSettingsLink),
        (Changed<Interaction>, With<interface::CreateGame>),
    >,
    game_settings: Query<(&CommonGameSettings, &BotGameSettings)>,
    mut game_data_ready: EventWriter<GameDataReady>,
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
            game_data_ready.send(GameDataReady::new_local(user, context_entity));
        }
    }
}

/// Get all data required to create a network game, spawn [`CreateGameContext`] component and
/// send [`GameDataReady`] event.
pub fn create_network_game(
    mut commands: Commands,
    button: Query<
        (&Interaction, &interface::GameSettingsLink),
        (Changed<Interaction>, With<interface::CreateGame>),
    >,
    game_settings: Query<(&CommonGameSettings, &NetworkGameSettings)>,
    client: Option<Res<grpc::GrpcClient>>,
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
            match client.create_game::<TicTacToe>(user, opponent) {
                Ok(task) => {
                    commands
                        .spawn(PendingNewGameBundle::<TicTacToe>::new())
                        .insert(task);
                    return;
                }
                Err(err) => println!("create_game call failed: {}", err),
            }
        }
    }
}

/// Whenever [`interface::SubmitPressed`] or [`TextInputSubmitEvent`] event is received
/// try to convert [`TextInputValue`] content into [`u64`] and
/// save result in a [`NetworkGameSettings`].
pub fn save_opponent(
    mut settings: Query<&mut NetworkGameSettings>,
    input: Query<(&TextInputValue, &interface::GameSettingsLink), With<interface::UserIdInput>>,
    mut submit_pressed: EventReader<interface::SubmitPressed>,
    mut text_input_submit: EventReader<TextInputSubmitEvent>,
) {
    let submit = |input: &str, settings: &mut NetworkGameSettings| {
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

/// Whenever [`interface::JoinPressed`] event is received:
/// if the game is already in memory send [`GameEntityReady`] event,
/// otherwise send GetGame request.
pub fn join_game(
    mut commands: Commands,
    game: Query<(Entity, &NetworkGame)>,
    mut join_pressed: EventReader<interface::JoinPressed>,
    mut game_entity_ready: EventWriter<GameEntityReady>,
    client: Option<Res<grpc::GrpcClient>>,
) {
    for event in join_pressed.read() {
        if let Some((game_entity, _)) = game.iter().find(|(_, &game)| *game == event.game_id()) {
            game_entity_ready.send(GameEntityReady::new(game_entity));
            return;
        }
        if let Some(client) = client.as_ref() {
            match client.get_game::<TicTacToe>(event.game_id()) {
                Ok(task) => {
                    commands
                        .spawn(PendingExistingGameBundle::<TicTacToe>::new(event.game_id()))
                        .insert(task);
                }
                Err(err) => println!("get_game call failed: {}", err),
            };
        }
    }
}
