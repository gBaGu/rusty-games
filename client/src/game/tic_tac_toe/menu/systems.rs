use bevy::prelude::*;
use game_server::core::tic_tac_toe::TicTacToe;

use crate::game::tic_tac_toe::bot::Strategy;
use crate::game::tic_tac_toe::components::{CreateGameContext, EnemyType};
use crate::game::{
    BotDifficulty, GameDataReady, GameEntityReady, NetworkGame, PendingExistingGameBundle,
    PendingNewGameBundle,
};
use crate::Settings;
use crate::{grpc, interface};

/// Whenever [`interface::GameSettingsContainer`] is added to a page layout fill it
/// with bot game settings interface.
/// Should be called only from state `AppState::Menu(MenuState::PlayAgainstBot)`
pub fn init_bot_settings_menu(
    mut commands: Commands,
    node: Query<Entity, Added<interface::GameSettingsContainer>>,
    asset_server: Res<AssetServer>,
) {
    for settings_entity in node.iter() {
        let text_font = interface::common::load_text_font(&asset_server);
        commands.entity(settings_entity).with_children(|builder| {
            builder.spawn(interface::TextBundle::new("Bot", text_font.clone()));
            builder
                .spawn(interface::CreateGameSettingBundle::<Strategy>::new_empty(
                    interface::common::row_node(),
                    settings_entity.into(),
                ))
                .with_children(|builder| {
                    let parent = builder.parent_entity();
                    [Strategy::Random, Strategy::QLearning]
                        .into_iter()
                        .map(|s| {
                            (
                                interface::SettingOptionButtonBundle::new(
                                    s,
                                    parent,
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
                .spawn(
                    interface::CreateGameSettingBundle::<BotDifficulty>::new_empty(
                        interface::common::row_node(),
                        settings_entity.into(),
                    ),
                )
                .with_children(|builder| {
                    let parent = builder.parent_entity();
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
                                parent,
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
/// Should be called only from state `AppState::Menu(MenuState::PlayOverNetwork)`
pub fn init_network_settings_menu(
    mut commands: Commands,
    node: Query<Entity, Added<interface::GameSettingsContainer>>,
    asset_server: Res<AssetServer>,
) {
    for settings_entity in node.iter() {
        let text_font = interface::common::load_text_font(&asset_server);
        let item_node = interface::common::menu_item_node();
        commands.entity(settings_entity).with_children(|builder| {
            builder
                .spawn(interface::CreateGameSettingBundle::<u64>::new_empty(
                    interface::common::row_node(),
                    settings_entity.into(),
                ))
                .with_children(|builder| {
                    let text = interface::TextBundle::new("Opponent id:", text_font.clone());
                    builder.spawn(item_node.clone()).with_child(text);
                    let input_id = builder
                        .spawn(interface::LocalSettingTextInputBundle::new(
                            item_node.clone(),
                            text_font.clone(),
                            builder.parent_entity(),
                        ))
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

/// Get all data required to create a bot game, spawn [`CreateGameContext`] component and
/// send [`GameDataReady`] event.
pub fn create_bot_game(
    mut commands: Commands,
    button: Query<
        (&Interaction, &interface::GameSettingsLink),
        (Changed<Interaction>, With<interface::CreateGame>),
    >,
    strategy_setting: Query<(
        &interface::LocalSetting<Strategy>,
        &interface::GameSettingsLink,
    )>,
    difficulty_setting: Query<(
        &interface::LocalSetting<BotDifficulty>,
        &interface::GameSettingsLink,
    )>,
    mut game_data_ready: EventWriter<GameDataReady>,
    settings: Res<Settings>,
) {
    for (interaction, _) in button.iter() {
        if *interaction == Interaction::Pressed {
            let Some(user) = settings.user_id() else {
                continue;
            };
            let Ok((strategy, _)) = strategy_setting.get_single() else {
                continue;
            };
            let Some(strategy) = **strategy else {
                continue;
            };
            let Ok((difficulty, _)) = difficulty_setting.get_single() else {
                continue;
            };
            if strategy.has_difficulty() {
                if difficulty.is_none() {
                    error!("bot difficulty is not set");
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

/// Get all data required to create a network game, spawn [`CreateGameContext`] component and
/// send [`GameDataReady`] event.
pub fn create_network_game(
    mut commands: Commands,
    button: Query<
        (&Interaction, &interface::GameSettingsLink),
        (Changed<Interaction>, With<interface::CreateGame>),
    >,
    opponent_setting: Query<(
        &interface::LocalSetting<u64>,
        &interface::GameSettingsLink,
    )>,
    client: Option<Res<grpc::GrpcClient>>,
    settings: Res<Settings>,
) {
    for (interaction, _) in button.iter() {
        if *interaction == Interaction::Pressed {
            let Some(user) = settings.user_id() else {
                continue;
            };
            let Ok((opponent, _)) = opponent_setting.get_single() else {
                continue;
            };
            let Some(opponent) = **opponent else {
                error!("opponent is not set");
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
                Err(err) => error!("get_game call failed: {}", err),
            };
        }
    }
}
