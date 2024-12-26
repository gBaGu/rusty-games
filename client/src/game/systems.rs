use bevy::prelude::*;
use game_server::core;
use game_server::proto;

use super::components::{FinishedGame, Game, LocalGame};
use super::pending_action::ConfirmationStatus;
use super::{
    ActionConfirmationFailed, ActiveGame, CurrentPlayer, CurrentUser, Draw, NetworkGame,
    PendingAction, PendingActionQueue, PlayerPosition, PlayerWon, ServerActionReceived,
    StateUpdated, TurnStart, UserAuthority, Winner,
};
use crate::{grpc, Settings};
use crate::grpc::CloseSession;
use crate::interface::{GameLeft, GameReady, GameReadyToExit};
use crate::UserIdChanged;

/// Watch local game entity creation and send [`GameReady`] event.
pub fn handle_local_game_creation(
    game: Query<Entity, (Added<Game>, Without<NetworkGame>)>,
    mut game_ready: EventWriter<GameReady>,
) {
    for game_entity in game.iter() {
        game_ready.send(GameReady::new(game_entity));
    }
}

/// Watch network game entity creation, initialize game session and
/// insert it to a game entity.
/// Send [`GameReady`] event upon success.
pub fn handle_network_game_creation<T>(
    mut commands: Commands,
    game: Query<(Entity, &NetworkGame), Added<LocalGame<T>>>,
    mut game_ready: EventWriter<GameReady>,
    client: Res<grpc::GrpcClient>,
    settings: Res<Settings>,
)
where
    T: core::Game + proto::GetGameType + Send + Sync + 'static,
    T::TurnData: Send,
{
    let Some(user) = settings.user_id() else {
        return;
    };
    for (game_entity, &network_game) in game.iter() {
        match client.game_session::<T>(*network_game, user) {
            Ok(session) => {
                commands.entity(game_entity).insert(session);
                game_ready.send(GameReady::new(game_entity));
            }
            Err(err) => println!("game_session call failed: {}", err),
        }
    }
}

/// If the game has [`PendingAction`] and it is not confirmed, send [`grpc::SendActionTask`] event
/// and change action status to `ConfirmationStatus::WaitingConfirmation`.
pub fn send_pending_action<T: Copy + Send + Sync + 'static>(
    mut game: Query<
        (Entity, &mut PendingActionQueue<T>),
        (With<ActiveGame>, Without<grpc::SendActionTask<T>>),
    >,
    mut send_action: EventWriter<grpc::SendSessionAction<T>>,
) {
    for (game_entity, mut queue) in game.iter_mut() {
        let Some(next_action) = queue.first_mut() else {
            continue;
        };
        if next_action.is_not_confirmed() {
            next_action.set_status(ConfirmationStatus::WaitingConfirmation);
            send_action.send(grpc::SendSessionAction::new(
                game_entity,
                *next_action.action(),
            ));
        }
    }
}

/// For all actions that are waiting for confirmation at the beginning of the [`PendingActionQueue`]
/// set the status to `ConfirmationStatus::NotConfirmed`.
pub fn revert_action_status<T: Send + Sync + 'static>(
    mut action_queue: Query<&mut PendingActionQueue<T>, With<ActiveGame>>,
    mut confirmation_failed: EventReader<ActionConfirmationFailed>,
) {
    for event in confirmation_failed.read() {
        let Ok(mut queue) = action_queue.get_mut(**event) else {
            continue;
        };
        println!("unable to confirm actions, set status to 'not confirmed'");
        // revert status for all actions for now
        for action in queue.iter_mut().take_while(|a| a.is_waiting_confirmation()) {
            action.set_status(ConfirmationStatus::NotConfirmed);
        }
    }
}

/// For every [`grpc::SessionFinished`] or [`grpc::SendSessionActionFailed`] event
/// send the [`ActionConfirmationFailed`].
pub fn handle_game_session_finished(
    mut session_finished: EventReader<grpc::SessionFinished>,
    mut send_action_failed: EventReader<grpc::SendSessionActionFailed>,
    mut confirmation_failed: EventWriter<ActionConfirmationFailed>,
) {
    for game_entity in session_finished
        .read()
        .map(|e| **e)
        .chain(send_action_failed.read().map(|e| **e))
    {
        confirmation_failed.send(ActionConfirmationFailed::new(game_entity));
    }
}

pub fn handle_game_session_update<T: Copy + Send + Sync + 'static>(
    mut update_received: EventReader<grpc::SessionUpdateReceived<T>>,
    mut server_action_received: EventWriter<ServerActionReceived<T>>,
    // mut game_session_error: EventWriter<>,
) {
    for event in update_received.read() {
        match event.update() {
            Ok(update) => {
                server_action_received.send(ServerActionReceived::new(
                    event.session_entity(),
                    update.player(),
                    *update.action(),
                ));
            }
            Err(err) => {
                // TODO: handle game session errors
                println!("game session error received: {}", err);
            }
        }
    }
}

pub fn handle_action_from_server<T: Copy + Send + Sync + 'static>(
    mut action_queue: Query<&mut PendingActionQueue<T>, With<ActiveGame>>,
    player: Query<(&PlayerPosition, &Parent), With<CurrentUser>>,
    mut server_action_received: EventReader<ServerActionReceived<T>>,
) {
    for event in server_action_received.read() {
        let Ok(mut queue) = action_queue.get_mut(event.game()) else {
            continue;
        };
        if player
            .iter()
            .filter(|(_, p)| p.get() == event.game())
            .find(|(&pos, _)| *pos == event.player())
            .is_some()
        {
            let Some(next_action) = queue.first_mut() else {
                continue;
            };
            if !next_action.is_waiting_confirmation() {
                println!(
                    "unexpected action status in a game queue: {:?}",
                    next_action.status()
                );
                continue;
            }
            next_action.set_status(ConfirmationStatus::Confirmed);
        } else {
            queue.push(PendingAction::new_confirmed(
                event.player(),
                *event.action(),
            ));
        }
    }
}

/// Receive [`StateUpdated`] event and send [`TurnStart`], [`PlayerWon`] or [`Draw`]
/// depending on a new state.
pub fn handle_state_updated(
    mut state_updated: EventReader<StateUpdated>,
    mut turn_start: EventWriter<TurnStart>,
    mut player_won: EventWriter<PlayerWon>,
    mut draw: EventWriter<Draw>,
) {
    for event in state_updated.read() {
        match event.state() {
            core::GameState::Turn(next_player) => {
                println!("turn start: {:?}", event.game());
                turn_start.send(TurnStart::new(event.game(), next_player));
            }
            core::GameState::Finished(core::FinishedState::Win(winner)) => {
                println!("win: {:?}", event.game());
                player_won.send(PlayerWon::new(event.game(), winner));
            }
            core::GameState::Finished(core::FinishedState::Draw) => {
                println!("draw: {:?}", event.game());
                draw.send(Draw::new(event.game()));
            }
        }
    }
}

/// Receives [`TurnStart`] event and updates [`CurrentPlayer`] accordingly.
pub fn update_current_player(
    mut commands: Commands,
    player: Query<(Entity, &PlayerPosition, &Parent)>,
    mut turn_start: EventReader<TurnStart>,
) {
    for event in turn_start.read() {
        player
            .iter()
            .filter(|(.., p)| event.game() == p.get())
            .for_each(|(player_entity, &position, _)| {
                let mut player_cmds = commands.entity(player_entity);
                if *position == event.player() {
                    println!("set current player: {}", event.player());
                    player_cmds.insert(CurrentPlayer);
                } else {
                    player_cmds.remove::<CurrentPlayer>();
                }
            });
    }
}

/// Receives [`Draw`] event, clears [`CurrentPlayer`] tag for players
/// and sends [`GameReadyToExit`] event because currently
/// there is nothing to do after the game is ended with a draw.
pub fn handle_draw(
    mut commands: Commands,
    player: Query<(Entity, &Parent), With<CurrentPlayer>>,
    mut draw: EventReader<Draw>,
    mut ready_to_exit: EventWriter<GameReadyToExit>,
) {
    for event in draw.read() {
        for (player_entity, _) in player.iter().filter(|(.., p)| p.get() == event.game()) {
            let mut player_cmds = commands.entity(player_entity);
            player_cmds.remove::<CurrentPlayer>();
        }
        ready_to_exit.send(GameReadyToExit::new(event.game()));
    }
}

/// Receives [`PlayerWon`] event, clears [`CurrentPlayer`] tag for players
/// and inserts [`Winner`] tag into the entity of a player that won the game.
pub fn handle_win(
    mut commands: Commands,
    player: Query<(Entity, &PlayerPosition, &Parent)>,
    mut player_won: EventReader<PlayerWon>,
) {
    for event in player_won.read() {
        player
            .iter()
            .filter(|(.., p)| p.get() == event.game())
            .for_each(|(player, &pos, _)| {
                let mut player_cmds = commands.entity(player);
                player_cmds.remove::<CurrentPlayer>();
                if event.player() == *pos {
                    player_cmds.insert(Winner);
                }
            })
    }
}

/// Listen for [`Draw`] and [`PlayerWon`] events and insert [`FinishedGame`] component
/// into game entity.
pub fn set_game_finished(
    mut commands: Commands,
    mut draw: EventReader<Draw>,
    mut player_won: EventReader<PlayerWon>,
) {
    for event in draw.read() {
        commands.entity(event.game()).insert(FinishedGame);
    }
    for event in player_won.read() {
        commands.entity(event.game()).insert(FinishedGame);
    }
}

/// Whenever user id is changed in settings insert/remove [`CurrentUser`] for every player
/// according to his id.
pub fn update_current_user(
    mut commands: Commands,
    player: Query<(Entity, &UserAuthority)>,
    mut user_id_changed: EventReader<UserIdChanged>,
) {
    if let Some(event) = user_id_changed.read().last() {
        for (player_entity, &user_authority) in player.iter() {
            let mut player_cmds = commands.entity(player_entity);
            if *user_authority == event.new_user_id() {
                player_cmds.insert(CurrentUser);
            } else {
                player_cmds.remove::<CurrentUser>();
            }
        }
    }
}

/// Listen for [`UserIdChanged`] event and if none of players ids matches new user id
/// then despawn game entity and its descendants.
pub fn clear_foreign_network_games(
    mut commands: Commands,
    game: Query<Entity, With<NetworkGame>>,
    player: Query<(&UserAuthority, &Parent)>,
    mut user_id_changed: EventReader<UserIdChanged>,
) {
    if let Some(event) = user_id_changed.read().last() {
        for game_entity in game.iter() {
            if player
                .iter()
                .filter(|(_, p)| p.get() == game_entity)
                .all(|(&user, _)| *user != event.new_user_id())
            {
                commands.entity(game_entity).despawn_recursive();
            }
        }
    }
}

/// Listen to [`GameLeft`] event and despawn game entity and its descendants
/// if one of the next conditions is met for a game:
/// - it is local (bot game);
/// - it is a network game and is finished.
pub fn clear_game_on_exit(
    mut commands: Commands,
    game: Query<(Option<&FinishedGame>, Option<&NetworkGame>), With<Game>>,
    mut game_left: EventReader<GameLeft>,
) {
    for event in game_left.read() {
        let Ok((finished_game, network_game)) = game.get(event.get()) else {
            continue;
        };
        let is_finished_game = finished_game.is_some();
        let is_network_game = network_game.is_some();
        if !is_network_game || (is_network_game && is_finished_game) {
            commands.entity(event.get()).despawn_recursive();
        }
    }
}

/// Whenever [`ActiveGame`] component is removed from entity, trigger the [`CloseSession`] event.
pub fn close_session(
    mut deactivated_game: RemovedComponents<ActiveGame>,
    mut close_session: EventWriter<CloseSession>,
) {
    for entity in deactivated_game.read() {
        close_session.send(CloseSession::new(entity));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::game::components::{
        CurrentUserPlayerBundle, LocalGameBundle, NetworkGameBundle, NetworkPlayerBundle,
    };
    use crate::Settings;
    use game_server::core;
    use game_server::core::tic_tac_toe::TicTacToe;

    type TTTLocalGameBundle = LocalGameBundle<TicTacToe, <TicTacToe as core::Game>::TurnData>;
    type TTTNetworkGameBundle = NetworkGameBundle<TicTacToe, <TicTacToe as core::Game>::TurnData>;

    #[test]
    fn game_entity_updated_on_finish() {
        let mut app = App::new();
        app.add_event::<Draw>();
        app.add_event::<PlayerWon>();
        app.add_systems(Update, set_game_finished);

        let game1 = app.world_mut().spawn(TTTLocalGameBundle::default()).id();
        let game2 = app.world_mut().spawn(TTTLocalGameBundle::default()).id();

        // neither of games has FinishedGame component
        assert!(!app.world().entity(game1).contains::<FinishedGame>());
        assert!(!app.world().entity(game2).contains::<FinishedGame>());

        // first game is finished with draw
        app.world_mut()
            .resource_mut::<Events<Draw>>()
            .send(Draw::new(game1));
        app.update();

        // only first game has FinishedGame component
        assert!(app.world().entity(game1).contains::<FinishedGame>());
        assert!(!app.world().entity(game2).contains::<FinishedGame>());

        // second game is finished with win
        app.world_mut()
            .resource_mut::<Events<PlayerWon>>()
            .send(PlayerWon::new(game2, 0));
        app.update();

        // both games now have FinishedGame component
        assert!(app.world().entity(game1).contains::<FinishedGame>());
        assert!(app.world().entity(game2).contains::<FinishedGame>());
    }

    #[test]
    fn user_id_change_clears_foreign_games() {
        let mut app = App::new();
        app.insert_resource(Settings::builder().user_id(1).build());
        app.add_event::<UserIdChanged>();
        app.add_systems(Update, clear_foreign_network_games);

        let make_spawner = |my_id, enemy_id| -> _ {
            move |builder: &mut WorldChildBuilder| {
                builder.spawn(CurrentUserPlayerBundle::new(my_id, 0));
                builder.spawn(NetworkPlayerBundle::new(enemy_id, 1));
            }
        };

        let game1_user1 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(1, TicTacToe::default()))
            .with_children(make_spawner(1, 2))
            .id();
        let game2_user1 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(2, TicTacToe::default()))
            .with_children(make_spawner(1, 3))
            .id();
        let game3_user2 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(3, TicTacToe::default()))
            .with_children(make_spawner(2, 4))
            .id();

        // simulate user id change
        app.world_mut()
            .resource_mut::<Events<UserIdChanged>>()
            .send(UserIdChanged::new(1));
        app.update();

        // game that doesn't have user 1 is despawned
        assert!(app.world().get::<Game>(game3_user2).is_none());

        // change user id
        app.world_mut().resource_mut::<Settings>().set_user_id(4);
        app.world_mut()
            .resource_mut::<Events<UserIdChanged>>()
            .send(UserIdChanged::new(4));
        app.update();

        // all games are cleared
        assert!(app.world().get::<Game>(game1_user1).is_none());
        assert!(app.world().get::<Game>(game2_user1).is_none());
        assert!(app.world().get::<Game>(game3_user2).is_none());
    }

    // GameLeft clears every local game and every finished network game
    #[test]
    fn game_left_event_clears_games() {
        let mut app = App::new();
        app.add_event::<GameLeft>();
        app.add_systems(Update, clear_game_on_exit);

        let game1 = app.world_mut().spawn(TTTLocalGameBundle::default()).id();
        let game2 = app.world_mut().spawn(TTTLocalGameBundle::default()).id();
        let game3 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(1, TicTacToe::default()))
            .insert(FinishedGame)
            .id();
        let game4 = app
            .world_mut()
            .spawn(TTTNetworkGameBundle::new(2, TicTacToe::default()))
            .id();

        // emit GameLeft events for games 1, 3, 4
        app.world_mut()
            .resource_mut::<Events<GameLeft>>()
            .send(GameLeft::new(game1));
        app.world_mut()
            .resource_mut::<Events<GameLeft>>()
            .send(GameLeft::new(game3));
        app.world_mut()
            .resource_mut::<Events<GameLeft>>()
            .send(GameLeft::new(game4));
        app.update();

        // games 1, 3 are despawned, 2 and 4 remain in the world
        // 2 because no event was fired for it
        // 4 because it is not finished
        assert!(app.world().get::<Game>(game1).is_none());
        assert!(app.world().get::<Game>(game2).is_some());
        assert!(app.world().get::<Game>(game3).is_none());
        assert!(app.world().get::<Game>(game4).is_some());
    }
}
