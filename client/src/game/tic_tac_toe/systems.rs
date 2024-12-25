use std::ops::Deref;

use bevy::prelude::*;
use game_server::core::tic_tac_toe::TicTacToe;
use game_server::core::{self, Game as _};
use game_server::proto;
use smallvec::SmallVec;

use super::bot::{BotBundle, NoDifficultyBotBundle};
use super::{
    Images, LocalGame, LocalGameBundle, NetworkGameBundle, PendingAction, PendingActionQueue,
    PlayerActionApplied, PlayerActionInitialized, O_SPRITE_PATH, X_SPRITE_PATH,
};
use crate::game::components::PendingGame;
use crate::game::resources::RefreshGameTimer;
use crate::game::tic_tac_toe::components::{CreateGameContext, EnemyType};
use crate::game::{
    ActiveGame, CurrentPlayer, CurrentUserPlayerBundle, FullGameInfo, GameDataReady, GameInfo,
    NetworkGame, NetworkPlayerBundle, StateUpdated, Winner,
};
use crate::grpc::RpcResultReady;
use crate::Settings;

/// Load X and O images and initialize [`Images`] resource.
pub fn init(mut commands: Commands, asset_server: Res<AssetServer>) {
    let x_img = asset_server.load(X_SPRITE_PATH);
    let o_img = asset_server.load(O_SPRITE_PATH);
    commands.insert_resource(Images::new(x_img, o_img));
}

/// Receive CreateGame rpc reply, despawn pending game entity, spawn [`CreateGameContext`]
/// component created from game info and send [`GameDataReady`] event.
pub fn handle_create_game_reply(
    mut commands: Commands,
    pending_game: Query<Entity, With<PendingGame<TicTacToe>>>,
    mut create_game_reply: EventReader<RpcResultReady<proto::CreateGameReply>>,
    mut game_data_ready: EventWriter<GameDataReady>,
    settings: Res<Settings>,
) {
    let Some(user) = settings.user_id() else {
        return;
    };
    for event in create_game_reply.read() {
        let Ok(pending_game_entity) = pending_game.get_single() else {
            continue;
        };
        commands.entity(pending_game_entity).despawn();
        match event.deref() {
            Ok(response) => {
                let Some(game_info) = &response.get_ref().game_info else {
                    println!("received empty CreateGame reply from server");
                    return;
                };
                let info = match GameInfo::try_from(game_info.clone()) {
                    Ok(info) => info,
                    Err(err) => {
                        println!("failed to decode game info: {}", err);
                        continue;
                    }
                };
                let game_id = info.id;
                if let Some(ctx) = CreateGameContext::new_from_game_info(user, info) {
                    let context_entity = commands.spawn(ctx).id();
                    game_data_ready.send(GameDataReady::new_over_network(
                        game_id,
                        user,
                        context_entity,
                    ));
                }
            }
            Err(err) => {
                println!("CreateGame request failed: {}", err);
            }
        }
    }
}

/// Receive GetGame rpc reply, despawn pending game entity, spawn [`CreateGameContext`] component
/// created from game info and send [`GameDataReady`] event.
pub fn handle_get_game_reply_on_join(
    mut commands: Commands,
    pending_game: Query<(Entity, &NetworkGame), With<PendingGame<TicTacToe>>>,
    mut get_game_reply: EventReader<RpcResultReady<proto::GetGameReply>>,
    mut game_data_ready: EventWriter<GameDataReady>,
    settings: Res<Settings>,
) {
    let Some(user) = settings.user_id() else {
        return;
    };
    for event in get_game_reply.read() {
        let Ok((pending_game_entity, &network_game)) = pending_game.get_single() else {
            continue;
        };
        commands.entity(pending_game_entity).despawn();
        match event.deref() {
            Ok(response) => {
                let Some(game_info) = &response.get_ref().game_info else {
                    println!("received empty CreateGame reply from server");
                    return;
                };
                let info = match FullGameInfo::try_from(game_info.clone()) {
                    Ok(info) => info,
                    Err(err) => {
                        println!("failed to decode game info: {}", err);
                        continue;
                    }
                };
                if info.info.id == *network_game {
                    if let Some(ctx) = CreateGameContext::new_from_full_game_info(user, info) {
                        let context_entity = commands.spawn(ctx).id();
                        game_data_ready.send(GameDataReady::new_over_network(
                            *network_game,
                            user,
                            context_entity,
                        ));
                    }
                }
            }
            Err(err) => {
                println!("CreateGame request failed: {}", err);
            }
        }
    }
}

/// Receive GetGame rpc reply and synchronize active game sending [`StateUpdated`] event if
/// the state has changed and [`PlayerActionApplied`] event for every new action.
pub fn handle_get_game(
    mut game: Query<(Entity, &NetworkGame, &mut LocalGame), With<ActiveGame>>,
    mut get_game_reply: EventReader<RpcResultReady<proto::GetGameReply>>,
    mut action_applied: EventWriter<PlayerActionApplied>,
    mut state_updated: EventWriter<StateUpdated>,
    mut timer: ResMut<RefreshGameTimer>,
) {
    let Ok((game_entity, &network_game, mut local_game)) = game.get_single_mut() else {
        return;
    };
    for event in get_game_reply.read() {
        timer.unpause();
        match event.deref() {
            Ok(response) => {
                let Some(info) = &response.get_ref().game_info else {
                    println!("dropping empty GetGame response");
                    continue;
                };
                if info.game_id != *network_game {
                    println!("received game info for other game, dropping");
                    continue;
                }
                let full_info = match FullGameInfo::try_from(info.clone()) {
                    Ok(info) => info,
                    Err(err) => {
                        println!("failed to decode full game info: {}", err);
                        continue;
                    }
                };
                let mut update_count = 0;
                for (i, row) in full_info.board.iter().enumerate() {
                    for (j, cell) in row.iter().enumerate() {
                        if let core::BoardCell(Some(player)) = cell {
                            let pos = core::GridIndex::new(i, j);
                            let local_cell = &mut local_game.board_mut()[pos];
                            if local_cell.is_none() {
                                *local_cell = *cell;
                                action_applied.send(PlayerActionApplied::new(
                                    game_entity,
                                    *player,
                                    pos,
                                ));
                                update_count += 1;
                            }
                        }
                    }
                }
                if update_count > 0 {
                    local_game.set_state(full_info.info.state);
                    state_updated.send(StateUpdated::new(game_entity, full_info.info.state));
                }
            }
            Err(err) => {
                println!("GetGame request failed: {}", err);
            }
        }
    }
}

/// Receive [`GameDataReady`] event, spawn game bundle and player bundles
/// as a children of a game entity, send [`GameReady`] event.
pub fn create(
    mut commands: Commands,
    context: Query<(Entity, &CreateGameContext)>,
    mut game_data_ready: EventReader<GameDataReady>,
) {
    for event in game_data_ready.read() {
        let Ok((ctx_entity, ctx)) = context.get(event.context()) else {
            continue;
        };
        commands.entity(ctx_entity).despawn();
        let game = ctx.game().cloned().unwrap_or_default();
        let state = game.state();

        let [user_position, enemy_position] = if ctx.user_first() { [0, 1] } else { [1, 0] };
        let user_id = commands
            .spawn(CurrentUserPlayerBundle::new(
                event.current_user(),
                user_position,
            ))
            .id();
        let enemy_id = match ctx.enemy() {
            EnemyType::User(id) => commands.spawn(NetworkPlayerBundle::new(id, enemy_position)),
            EnemyType::Bot {
                strategy,
                difficulty,
            } => {
                let bot_id = 0; // TODO: manage bot ids in case of multiple bots
                if let Some(difficulty) = difficulty {
                    commands.spawn(BotBundle::new(bot_id, enemy_position, strategy, difficulty))
                } else {
                    commands.spawn(NoDifficultyBotBundle::new(bot_id, enemy_position, strategy))
                }
            }
        }
        .id();
        match state {
            core::GameState::Turn(player) => {
                if player == user_position {
                    commands.entity(user_id).insert(CurrentPlayer);
                } else if player == enemy_position {
                    commands.entity(enemy_id).insert(CurrentPlayer);
                }
            }
            core::GameState::Finished(core::FinishedState::Win(player)) => {
                if player == user_position {
                    commands.entity(user_id).insert(Winner);
                } else if player == enemy_position {
                    commands.entity(enemy_id).insert(Winner);
                }
            }
            _ => {}
        };

        let mut game_cmds = match event.id() {
            Some(id) => commands.spawn(NetworkGameBundle::new(id, game)),
            None => commands.spawn(LocalGameBundle::from(game)),
        };
        let game_entity = game_cmds.id();
        game_cmds.insert_children(0, &[user_id, enemy_id]);
        println!("game created: {:?}", game_entity);
    }
}

/// Receive [`PlayerActionInitialized`] event and if the game has no [`PendingAction`]
/// insert [`PendingActionBundle`] into a game entity.
/// If game entity contains [`NetworkGame`] component pending action will be
/// `PendingActionStatus::NotConfirmed`, otherwise `PendingActionStatus::Confirmed`.
pub fn create_pending_action(
    mut game: Query<(Option<&NetworkGame>, &mut PendingActionQueue), With<ActiveGame>>,
    mut action_initialized: EventReader<PlayerActionInitialized>,
) {
    for event in action_initialized.read() {
        match game.get_mut(event.game()) {
            Ok((Some(_), mut queue)) => {
                queue.push(PendingAction::new_unconfirmed(
                    event.player(),
                    event.action(),
                ));
            }
            Ok((None, mut queue)) => {
                queue.push(PendingAction::new_confirmed(event.player(), event.action()));
            }
            Err(err) => {
                println!("failed to get game entity: {}", err);
                continue;
            }
        };
    }
}

/// If the game has [`PendingAction`] and it is confirmed, apply it and send
/// [`PlayerActionApplied`] and [`StateUpdated`] events.
pub fn apply_confirmed(
    mut game: Query<(Entity, &mut LocalGame, &mut PendingActionQueue), With<ActiveGame>>,
    mut action_applied: EventWriter<PlayerActionApplied>,
    mut state_updated: EventWriter<StateUpdated>,
) {
    for (game_entity, mut game, mut queue) in game.iter_mut() {
        let mut actions_processed = 0;
        for pending_action in queue.iter().take_while(|action| action.is_confirmed()) {
            match game.update(pending_action.player(), *pending_action.action()) {
                Ok(state) => {
                    action_applied.send(PlayerActionApplied::new(
                        game_entity,
                        pending_action.player(),
                        *pending_action.action(),
                    ));
                    state_updated.send(StateUpdated::new(game_entity, state));
                }
                Err(err) => println!("action {:?} failed with {}", pending_action, err),
            }
            actions_processed += 1;
        }
        if actions_processed > 0 {
            *queue = PendingActionQueue::from(SmallVec::from(&queue[actions_processed..]));
        }
    }
}
