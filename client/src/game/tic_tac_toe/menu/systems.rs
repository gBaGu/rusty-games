use bevy::prelude::*;
use game_server::core::tic_tac_toe::TicTacToe;

use super::{BotGameSettings, NetworkGameSettings, StrategyUpdated};
use crate::game::tic_tac_toe::bot;
use crate::game::tic_tac_toe::components::{CreateGameContext, EnemyType};
use crate::game::{
    BotDifficulty, GameDataReady, GameEntityReady, NetworkGame, PendingExistingGameBundle,
    PendingNewGameBundle,
};
use crate::util::watched_value::{ValueUpdated, WatchedValue, WatchedValueBundle};
use crate::Settings;
use crate::{grpc, interface};

/// Whenever [`interface::GameSettingsContainer`] is added to a page layout fill it
/// with bot game settings interface.  
/// Spawns [`BotGameSettings`] with child [`WatchedValueBundle`] for every setting it has.
/// Should be called only from state `AppState::Menu(MenuState::PlayAgainstBot)`
pub fn init_bot_settings_menu(
    mut commands: Commands,
    node: Query<Entity, Added<interface::GameSettingsContainer>>,
    asset_server: Res<AssetServer>,
) {
    for settings_entity in node.iter() {
        let strategy = commands
            .spawn(WatchedValueBundle::<bot::Strategy>::default())
            .id();
        let difficulty = commands
            .spawn(WatchedValueBundle::<BotDifficulty>::default())
            .id();
        commands
            .spawn(BotGameSettings::new(strategy, difficulty))
            .insert_children(0, &[strategy, difficulty]);

        let text_font = interface::common::load_text_font(&asset_server);
        commands.entity(settings_entity).with_children(|builder| {
            builder.spawn(interface::TextBundle::new("Bot", text_font.clone()));
            builder
                .spawn(interface::common::row_node())
                .with_children(|builder| {
                    [bot::Strategy::Random, bot::Strategy::QLearning]
                        .into_iter()
                        .map(|s| {
                            (
                                interface::SettingOptionButtonBundle::new(
                                    s,
                                    strategy,
                                    Val::Px(2.),
                                    true,
                                ),
                                interface::TextBundle::new(s.to_string(), text_font.clone()),
                            )
                        })
                        .for_each(|(button, text)| {
                            builder.spawn(button).with_child(text);
                        });
                });
            builder
                .spawn(interface::common::row_node())
                .with_children(|builder| {
                    [
                        BotDifficulty::Easy,
                        BotDifficulty::Medium,
                        BotDifficulty::Hard,
                    ]
                    .into_iter()
                    .map(|d| {
                        (
                            interface::SettingOptionButtonBundle::new(
                                d,
                                difficulty,
                                Val::Px(2.),
                                false,
                            ),
                            interface::TextBundle::new(d.to_string(), text_font.clone()),
                        )
                    })
                    .for_each(|(button, text)| {
                        builder.spawn(button).with_child(text);
                    });
                });
        });
    }
}

/// Whenever [`interface::GameSettingsContainer`] is added to a page layout fill it
/// with network game settings interface.
/// Spawns [`NetworkGameSettings`] with child [`WatchedValueBundle`] for every setting it has.
/// Should be called only from state `AppState::Menu(MenuState::PlayOverNetwork)`
pub fn init_network_settings_menu(
    mut commands: Commands,
    node: Query<Entity, Added<interface::GameSettingsContainer>>,
    asset_server: Res<AssetServer>,
) {
    for settings_entity in node.iter() {
        let opponent = commands.spawn(WatchedValueBundle::<u64>::default()).id();
        commands
            .spawn(NetworkGameSettings::new(opponent))
            .insert_children(0, &[opponent]);

        let text_font = interface::common::load_text_font(&asset_server);
        let item_node = interface::common::menu_item_node();
        commands.entity(settings_entity).with_children(|builder| {
            builder
                .spawn(interface::common::row_node())
                .with_children(|builder| {
                    let text = interface::TextBundle::new("Opponent id:", text_font.clone());
                    builder.spawn(item_node.clone()).with_child(text);
                    let input_id = builder
                        .spawn(interface::TextInputBundle::new(
                            item_node.clone(),
                            text_font.clone(),
                            opponent,
                        ))
                        .id();
                    builder
                        .spawn(interface::SubmitButtonBundle::new(item_node, input_id))
                        .with_child(interface::TextBundle::new("Save", text_font.clone()));
                });
        });
    }
}

/// Clean up [`BotGameSettings`], [`NetworkGameSettings`] entities and their children.
pub fn cleanup(
    mut commands: Commands,
    bot_settings: Query<Entity, With<BotGameSettings>>,
    network_settings: Query<Entity, With<NetworkGameSettings>>,
) {
    for entity in bot_settings.iter().chain(network_settings.iter()) {
        commands.entity(entity).despawn_recursive();
    }
}

/// Receive [`ValueUpdated`] event for strategy setting and send [`StrategyUpdated`] event
/// with a list of difficulty options for the new strategy.
pub fn get_bot_strategy_difficulty_options(
    mut setting_updated: EventReader<ValueUpdated<bot::Strategy>>,
    mut strategy_updated: EventWriter<StrategyUpdated>,
    q_learning_model: Res<bot::QLearningModel>,
) {
    for event in setting_updated.read() {
        let difficulties = match event.value() {
            Some(bot::Strategy::QLearning) => q_learning_model.get_available_difficulties(),
            _ => smallvec::smallvec![],
        };
        strategy_updated.send(StrategyUpdated::new(event.source(), difficulties));
    }
}

/// Receive [`StrategyUpdated`] event and if current difficulty setting isn't supported
/// by new strategy reset it.
pub fn update_bot_difficulty_setting(
    mut difficulty_setting: Query<(&mut WatchedValue<BotDifficulty>, &Parent)>,
    strategy_setting: Query<&Parent, With<WatchedValue<bot::Strategy>>>,
    mut strategy_updated: EventReader<StrategyUpdated>,
) {
    for event in strategy_updated.read() {
        let Ok(parent) = strategy_setting.get(event.setting()) else {
            continue;
        };
        let Some((mut difficulty, _)) = difficulty_setting.iter_mut().find(|(_, p)| *p == parent)
        else {
            continue;
        };
        if let Some(current_difficulty) = (*difficulty).as_ref() {
            if !event.difficulty_supported(current_difficulty) {
                difficulty.reset();
            }
        }
    }
}

/// Receive [`StrategyUpdated`] event and update difficulty buttons [`Visibility`] according to
/// supported difficulties for the new strategy.
pub fn update_difficulty_buttons_visibility(
    mut difficulty_button: Query<(&mut Visibility, &BotDifficulty, &interface::StorageLink)>,
    difficulty_setting: Query<(Entity, &Parent), With<WatchedValue<BotDifficulty>>>,
    strategy_setting: Query<&Parent, With<WatchedValue<bot::Strategy>>>,
    mut strategy_updated: EventReader<StrategyUpdated>,
) {
    for event in strategy_updated.read() {
        let Ok(parent) = strategy_setting.get(event.setting()) else {
            continue;
        };
        let Some((difficulty_entity, _)) = difficulty_setting.iter().find(|(_, p)| *p == parent)
        else {
            continue;
        };
        for (mut visibility, difficulty, _) in difficulty_button
            .iter_mut()
            .filter(|(.., link)| link.get() == difficulty_entity)
        {
            *visibility = if event.difficulty_supported(difficulty) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
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
                            error!("get_player_games call failed: {}", err);
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
                error!("get_player_games call failed: {}", err);
                *list = interface::GameList::Message("Server is down".into());
            }
        }
    }
}

/// Get [`BotGameSettings`], find all settings required to create a bot game,
/// spawn [`CreateGameContext`] component and send [`GameDataReady`] event.
pub fn create_bot_game(
    mut commands: Commands,
    button: Query<&Interaction, (Changed<Interaction>, With<interface::CreateGame>)>,
    bot_settings: Query<&BotGameSettings>,
    strategy_setting: Query<(Entity, &WatchedValue<bot::Strategy>)>,
    difficulty_setting: Query<(Entity, &WatchedValue<BotDifficulty>)>,
    mut game_data_ready: EventWriter<GameDataReady>,
    settings: Res<Settings>,
) {
    for interaction in button.iter() {
        if *interaction == Interaction::Pressed {
            let Some(user) = settings.user_id() else {
                continue;
            };
            let Ok(settings) = bot_settings.get_single() else {
                continue;
            };
            let Some((_, strategy)) = strategy_setting
                .iter()
                .find(|(e, _)| *e == settings.strategy())
            else {
                continue;
            };
            let Some(strategy) = **strategy else {
                error!("failed to create game: strategy is not set");
                continue;
            };
            let Some((_, difficulty)) = difficulty_setting
                .iter()
                .find(|(e, _)| *e == settings.difficulty())
            else {
                continue;
            };
            if strategy.has_difficulty() {
                if difficulty.is_none() {
                    error!("failed to create game: difficulty is not set");
                    continue;
                }
            }

            let ctx = CreateGameContext::new(
                EnemyType::Bot {
                    strategy,
                    difficulty: **difficulty,
                },
                true,
                None,
            );
            let context_entity = commands.spawn(ctx).id();
            game_data_ready.send(GameDataReady::new_local(user, context_entity));
        }
    }
}

/// Get [`NetworkGameSettings`], find all settings required to create a network game,
/// spawn [`CreateGameContext`] component and send [`GameDataReady`] event.
pub fn create_network_game(
    mut commands: Commands,
    button: Query<&Interaction, (Changed<Interaction>, With<interface::CreateGame>)>,
    network_settings: Query<&NetworkGameSettings>,
    opponent_setting: Query<(Entity, &WatchedValue<u64>)>,
    client: Option<Res<grpc::GrpcClient>>,
    settings: Res<Settings>,
) {
    for interaction in button.iter() {
        if *interaction == Interaction::Pressed {
            let Some(user) = settings.user_id() else {
                continue;
            };
            let Ok(settings) = network_settings.get_single() else {
                continue;
            };
            let Some((_, opponent)) = opponent_setting
                .iter()
                .find(|(e, _)| *e == settings.opponent())
            else {
                continue;
            };
            let Some(opponent) = **opponent else {
                error!("failed to create game: opponent is not set");
                continue;
            };
            let Some(client) = client.as_ref() else {
                error!("grpc client is not connected");
                continue;
            };
            match client.create_game::<TicTacToe>(user, opponent) {
                Ok(task) => {
                    commands
                        .spawn(PendingNewGameBundle::<TicTacToe>::new())
                        .insert(task);
                    return;
                }
                Err(err) => error!("create_game call failed: {}", err),
            }
        }
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
            game_entity_ready.send(game_entity.into());
            return;
        }
        if let Some(client) = client.as_ref() {
            match client.get_game::<TicTacToe>(event.game_id()) {
                Ok(task) => {
                    commands
                        .spawn(PendingExistingGameBundle::<TicTacToe>::new(event.game_id()))
                        .insert(task);
                }
                Err(err) => error!("get_game call failed: {}", err),
            };
        }
    }
}
