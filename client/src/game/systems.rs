use std::fmt;

use bevy::prelude::*;
use game_server::{core, proto};
use smallvec::SmallVec;

use super::components::{ActionResendTimer, FinishedGame, Game, LocalGame};
use super::pending_action::ConfirmationStatus;
use super::{
    ActionConfirmationFailed, ActiveGame, CurrentPlayer, CurrentUser, Draw, GameEntityReady,
    NetworkGame, PendingAction, PendingActionQueue, PlayerPosition, PlayerWon, StateUpdated,
    TurnStart, UserAuthority, Winner,
};
use crate::game::events::{
    ActionApplied, ActionConfirmed, ActionDropped, ActionEnqueued, ActionInitialized,
    ActionQueueNextChanged, ActionStatusRevertPolicy,
};
use crate::UserIdChanged;
use crate::{grpc, interface};

/// Watch the game entity creation and send [`GameEntityReady`] event.
pub fn handle_game_spawn(
    game: Query<Entity, Added<Game>>,
    mut game_entity_ready: EventWriter<GameEntityReady>,
) {
    for game_entity in game.iter() {
        game_entity_ready.send(game_entity.into());
    }
}

/// Receive the [`GameEntityReady`] event and in case of a local game
/// send [`interface::GameReady`] event.
pub fn handle_local_game_creation(
    local_game: Query<(), (With<Game>, Without<NetworkGame>)>,
    mut game_entity_ready: EventReader<GameEntityReady>,
    mut game_ready: EventWriter<interface::GameReady>,
) {
    for event in game_entity_ready.read() {
        if local_game.contains(event.get()) {
            game_ready.send(event.get().into());
        }
    }
}

/// Receive the [`GameEntityReady`] event and in case of a network game
/// trigger session initialization.
pub fn initialize_game_session<T>(
    network_game: Query<(), With<NetworkGame>>,
    mut game_entity_ready: EventReader<GameEntityReady>,
    mut open_session: EventWriter<grpc::OpenSession>,
) where
    T: core::Game + proto::GetGameType + Send + Sync + 'static,
    T::TurnData: Send,
{
    for event in game_entity_ready.read() {
        if network_game.contains(event.get()) {
            open_session.send(grpc::OpenSession::new(event.get()));
        }
    }
}

/// Receive the [`grpc::SessionOpened`] event and in case if entity it contains
/// is a network game and is not active (which means this game is being initialized)
/// send [`interface::GameReady`] event.
pub fn network_game_initialization_finished(
    network_game: Query<(), (With<NetworkGame>, Without<ActiveGame>)>,
    mut session_opened: EventReader<grpc::SessionOpened>,
    mut game_ready: EventWriter<interface::GameReady>,
) {
    for event in session_opened.read() {
        if network_game.contains(event.get()) {
            info!("session is initialized for game {}", event.get());
            game_ready.send(event.get().into());
        }
    }
}

/// Receive [`ActionEnqueued`] and [`ActionDropped`] events and
/// send [`ActionQueueNextChanged`] when the first action in the queue has changed.
pub fn action_queue_next_changed<T: Send + Sync + 'static>(
    queue: Query<&PendingActionQueue<T>>,
    mut action_enqueued: EventReader<ActionEnqueued<T>>,
    mut action_dropped: EventReader<ActionDropped<T>>,
    mut next_changed: EventWriter<ActionQueueNextChanged>,
) {
    let mut dropped = SmallVec::<[_; 8]>::new();
    for game_entity in action_dropped.read().map(|e| e.game()) {
        if !dropped.contains(&game_entity) {
            next_changed.send(game_entity.into());
            dropped.push(game_entity);
        }
    }
    let mut enqueued = SmallVec::<[_; 8]>::new();
    for event in action_enqueued.read() {
        if let Some((_, n)) = enqueued.iter_mut().find(|(e, _)| *e == event.game()) {
            *n += 1;
        } else {
            enqueued.push((event.game(), 1usize));
        }
    }
    for (game_entity, new_actions) in enqueued {
        if dropped.contains(&game_entity) {
            continue;
        }
        if matches!(queue.get(game_entity), Ok(queue) if queue.len() == new_actions) {
            next_changed.send(game_entity.into());
        }
    }
}

/// Receive [`ActionInitialized`] event and insert unconfirmed [`PendingAction`]
/// into a [`PendingActionQueue`] of a game entity received in the event.  
/// Triggers [`ActionEnqueued`].
pub fn create_pending_action<T: fmt::Display + Copy + Send + Sync + 'static>(
    mut game: Query<&mut PendingActionQueue<T>, With<ActiveGame>>,
    mut action_initialized: EventReader<ActionInitialized<T>>,
    mut action_enqueued: EventWriter<ActionEnqueued<T>>,
) {
    for event in action_initialized.read() {
        let action = *event.action();
        let player = event.player();
        info!(
            "game {} action {} initialized, player={}",
            event.game(),
            action,
            player
        );
        let Ok(mut queue) = game.get_mut(event.game()) else {
            continue;
        };
        queue.push(PendingAction::new_unconfirmed(player, action));
        action_enqueued.send(ActionEnqueued::new(event.game(), player, action));
    }
}

/// Receive [`ActionEnqueued`] and if the game is local confirm all pending actions.
/// Triggers [`ActionConfirmed`].
pub fn confirm_local_game_action<T: Copy + Send + Sync + 'static>(
    mut game: Query<&mut PendingActionQueue<T>, (With<ActiveGame>, Without<NetworkGame>)>,
    mut action_enqueued: EventReader<ActionEnqueued<T>>,
    mut action_confirmed: EventWriter<ActionConfirmed<T>>,
) {
    for event in action_enqueued.read() {
        let Ok(mut queue) = game.get_mut(event.game()) else {
            continue;
        };
        for action in queue.confirm_latest() {
            action_confirmed.send(ActionConfirmed::new(
                event.game(),
                action.player(),
                *action.action(),
            ));
        }
    }
}

/// Listen to [`ActionQueueNextChanged`] event or [`ActionResendTimer`] removal and
/// if the first pending action in the queue is not confirmed
/// send it in a [`grpc::SessionActionReadyToSend`] event
/// and change status to `ConfirmationStatus::WaitingConfirmation`.
pub fn send_pending_action<T: Copy + Send + Sync + 'static>(
    mut game: Query<
        &mut PendingActionQueue<T>,
        (
            With<ActiveGame>,
            With<NetworkGame>,
            Without<ActionResendTimer>,
        ),
    >,
    mut next_changed: EventReader<ActionQueueNextChanged>,
    mut resend: RemovedComponents<ActionResendTimer>,
    mut action_ready: EventWriter<grpc::SessionActionReadyToSend<T>>,
) {
    for game_entity in resend.read().chain(next_changed.read().map(|e| e.get())) {
        let Ok(mut queue) = game.get_mut(game_entity) else {
            continue;
        };
        let Some(next_action) = queue.first_mut() else {
            continue;
        };
        if next_action.is_not_confirmed() {
            next_action.set_status(ConfirmationStatus::WaitingConfirmation);
            action_ready.send(grpc::SessionActionReadyToSend::new(
                game_entity,
                *next_action.action(),
            ));
        }
    }
}

/// Insert [`ActionResendTimer`] into the entity when [`ActionConfirmationFailed`] is received.
pub fn create_resend_action_timer<T: Send + Sync + 'static>(
    mut commands: Commands,
    game: Query<(), (With<ActiveGame>, Without<ActionResendTimer>)>,
    mut confirmation_failed: EventReader<ActionConfirmationFailed<T>>,
) {
    for event in confirmation_failed.read() {
        if game.contains(event.game()) {
            commands
                .entity(event.game())
                .insert(ActionResendTimer::default());
        }
    }
}

/// Tick [`ActionResendTimer`] and remove it when it's finished.
pub fn resend_action_timer_tick(
    mut commands: Commands,
    mut timer: Query<(Entity, &mut ActionResendTimer)>,
    time: Res<Time>,
) {
    for (timer_entity, mut timer) in timer.iter_mut() {
        if timer.tick(time.delta()).just_finished() {
            commands.entity(timer_entity).remove::<ActionResendTimer>();
        }
    }
}

/// For every [`ActionConfirmationFailed`] event
/// set the status to `ConfirmationStatus::NotConfirmed` for action or actions
/// depending on the [`ActionStatusRevertPolicy`] value in the event.
pub fn revert_action_status<T: PartialEq + Send + Sync + 'static>(
    mut action_queue: Query<&mut PendingActionQueue<T>, With<ActiveGame>>,
    mut confirmation_failed: EventReader<ActionConfirmationFailed<T>>,
) {
    for event in confirmation_failed.read() {
        let Ok(mut queue) = action_queue.get_mut(event.game()) else {
            continue;
        };
        warn!("unable to confirm actions for game {}", event.game());
        match event.revert_policy() {
            ActionStatusRevertPolicy::All => {
                for action in queue.iter_mut().filter(|a| a.is_waiting_confirmation()) {
                    action.set_status(ConfirmationStatus::NotConfirmed);
                }
            }
            ActionStatusRevertPolicy::Single(action) => {
                if let Some(action) = queue
                    .iter_mut()
                    .find(|a| a.is_waiting_confirmation() && a.action() == action)
                {
                    action.set_status(ConfirmationStatus::NotConfirmed);
                }
            }
        }
    }
}

/// For every [`grpc::SessionClosed`] event trigger status revert for all actions that
/// are waiting for confirmation.
/// For every [`grpc::SessionActionSendFailed`] event trigger status revert for
/// the action contained in the event.
/// In case when one entity will have both events [`grpc::SessionActionSendFailed`] will be ignored.
pub fn action_confirmation_failed<T: Copy + Send + Sync + 'static>(
    mut session_closed: EventReader<grpc::SessionClosed>,
    mut action_send_failed: EventReader<grpc::SessionActionSendFailed<T>>,
    mut confirmation_failed: EventWriter<ActionConfirmationFailed<T>>,
) {
    let closed_sessions =
        SmallVec::<[_; 8]>::from_iter(session_closed.read().map(|e| e.get()).inspect(|e| {
            confirmation_failed.send(ActionConfirmationFailed::revert_all(*e));
        }));
    for event in action_send_failed.read() {
        if closed_sessions.contains(&event.session_entity()) {
            continue;
        }
        confirmation_failed.send(ActionConfirmationFailed::revert_single(
            event.session_entity(),
            *event.action(),
        ));
    }
}

/// Receive [`grpc::SessionUpdateReceived`] event and find [`PendingActionQueue`].
/// If the action received in the event is a current player action
/// then set its status to `ConfirmationStatus::Confirmed`,
/// otherwise push confirmed action to the queue and send [`ActionEnqueued`] event.
/// In both cases send [`ActionConfirmed`] event.
pub fn handle_action_from_server<T: Copy + Send + Sync + 'static>(
    mut action_queue: Query<&mut PendingActionQueue<T>, With<ActiveGame>>,
    player: Query<(&PlayerPosition, &Parent), With<CurrentUser>>,
    mut update_received: EventReader<grpc::SessionUpdateReceived<T>>,
    mut action_enqueued: EventWriter<ActionEnqueued<T>>,
    mut action_confirmed: EventWriter<ActionConfirmed<T>>,
) {
    for event in update_received.read() {
        let Ok(mut queue) = action_queue.get_mut(event.session_entity()) else {
            continue;
        };
        if player
            .iter()
            .filter(|(_, p)| p.get() == event.session_entity())
            .find(|(&pos, _)| *pos == event.player())
            .is_some()
        {
            let Some(next_action) = queue.first_mut() else {
                continue;
            };
            if !next_action.is_waiting_confirmation() {
                error!("unexpected pending action status: {}", next_action.status());
                continue;
            }
            if next_action.player() != event.player() {
                error!("unexpected pending action player: {}", next_action.player());
                continue;
            }
            next_action.set_status(ConfirmationStatus::Confirmed);
        } else {
            queue.push(PendingAction::new_confirmed(
                event.player(),
                *event.action(),
            ));
            action_enqueued.send(ActionEnqueued::new(
                event.session_entity(),
                event.player(),
                *event.action(),
            ));
        }
        action_confirmed.send(ActionConfirmed::new(
            event.session_entity(),
            event.player(),
            *event.action(),
        ));
    }
}

/// Whenever action is confirmed or next action in the queue is changed
/// take first consecutive confirmed actions from [`PendingActionQueue`],
/// apply them and send [`ActionApplied`] and [`StateUpdated`] events.  
/// In case of an error drop action and send [`ActionDropped`] event.
pub fn apply_confirmed<T>(
    mut game: Query<(&mut LocalGame<T>, &mut PendingActionQueue<T::TurnData>), With<ActiveGame>>,
    mut action_confirmed: EventReader<ActionConfirmed<T::TurnData>>,
    mut next_changed: EventReader<ActionQueueNextChanged>,
    mut action_applied: EventWriter<ActionApplied<T::TurnData>>,
    mut state_updated: EventWriter<StateUpdated>,
    mut action_dropped: EventWriter<ActionDropped<T::TurnData>>,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Copy + Send + Sync + 'static,
{
    let to_confirm = action_confirmed
        .read()
        .map(|e| e.game())
        .chain(next_changed.read().map(|e| e.get()));
    for game_entity in to_confirm {
        let Ok((mut game, mut queue)) = game.get_mut(game_entity) else {
            continue;
        };
        for pending_action in queue.pop_confirmed() {
            match game.update(pending_action.player(), *pending_action.action()) {
                Ok(state) => {
                    action_applied.send(ActionApplied::new(
                        game_entity,
                        pending_action.player(),
                        *pending_action.action(),
                    ));
                    state_updated.send(StateUpdated::new(game_entity, state));
                }
                Err(err) => {
                    action_dropped.send(ActionDropped::new(
                        game_entity,
                        pending_action.player(),
                        *pending_action.action(),
                        format!("game update failed after confirmation: {}", err),
                    ));
                }
            }
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
        info!("game {} state updated: {:?}", event.game(), event.state());
        match event.state() {
            core::GameState::Turn(next_player) => {
                turn_start.send(TurnStart::new(event.game(), next_player));
            }
            core::GameState::Finished(core::FinishedState::Win(winner)) => {
                player_won.send(PlayerWon::new(event.game(), winner));
            }
            core::GameState::Finished(core::FinishedState::Draw) => {
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
                    player_cmds.insert(CurrentPlayer);
                } else {
                    player_cmds.remove::<CurrentPlayer>();
                }
            });
    }
}

/// Receives [`Draw`] event, clears [`CurrentPlayer`] tag for players
/// and sends [`interface::GameReadyToExit`] event because currently
/// there is nothing to do after the game is ended with a draw.
pub fn handle_draw(
    mut commands: Commands,
    player: Query<(Entity, &Parent), With<CurrentPlayer>>,
    mut draw: EventReader<Draw>,
    mut ready_to_exit: EventWriter<interface::GameReadyToExit>,
) {
    for event in draw.read() {
        for (player_entity, _) in player.iter().filter(|(.., p)| p.get() == event.game()) {
            let mut player_cmds = commands.entity(player_entity);
            player_cmds.remove::<CurrentPlayer>();
        }
        ready_to_exit.send(event.game().into());
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
            if matches!(event.new_user_id(), Some(id) if id == *user_authority) {
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
                .all(|(&user, _)| !matches!(event.new_user_id(), Some(id) if id == *user))
            {
                commands.entity(game_entity).despawn_recursive();
            }
        }
    }
}

/// Listen to [`interface::GameLeft`] event and despawn game entity and its descendants
/// if one of the next conditions is met for a game:
/// - it is local (bot game);
/// - it is a network game and is finished.
pub fn clear_game_on_exit(
    mut commands: Commands,
    game: Query<(Option<&FinishedGame>, Option<&NetworkGame>), With<Game>>,
    mut game_left: EventReader<interface::GameLeft>,
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

/// Whenever [`ActiveGame`] component is removed from entity,
/// trigger the [`grpc::CloseSession`] event.
pub fn close_session(
    mut deactivated_game: RemovedComponents<ActiveGame>,
    mut close_session: EventWriter<grpc::CloseSession>,
) {
    for entity in deactivated_game.read() {
        close_session.send(entity.into());
    }
}

/// Log [`PendingAction`] when it's dropped out of the queue.
pub fn log_dropped_action<T: fmt::Display + Send + Sync + 'static>(
    mut action_dropped: EventReader<ActionDropped<T>>,
) {
    for event in action_dropped.read() {
        warn!(
            "game {} action {} dropped, player={}, reason={}",
            event.game(),
            event.action(),
            event.player(),
            event.reason(),
        );
    }
}

/// Log [`PendingAction`] when it's added to the queue.
pub fn log_enqueued_action<T: fmt::Display + Send + Sync + 'static>(
    mut action_enqueued: EventReader<ActionEnqueued<T>>,
) {
    for event in action_enqueued.read() {
        debug!(
            "game {} action {} added to queue, player={}",
            event.game(),
            event.action(),
            event.player(),
        );
    }
}

/// Log [`PendingAction`] when it was confirmed.
pub fn log_confirmed_action<T: fmt::Display + Send + Sync + 'static>(
    mut action_confirmed: EventReader<ActionConfirmed<T>>,
) {
    for event in action_confirmed.read() {
        info!(
            "game {} action {} confirmed, player={}",
            event.game(),
            event.action(),
            event.player(),
        );
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

    fn clear_events<E: Event>(w: &mut World) {
        w.resource_mut::<Events<E>>().clear();
    }

    fn get_component<T: Component>(w: &World, e: Entity) -> &T {
        w.entity(e).get::<T>().unwrap()
    }

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

    /// GameLeft clears every local game and every finished network game
    #[test]
    fn game_left_event_clears_games() {
        use interface::GameLeft;

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
        let mut events = app.world_mut().resource_mut::<Events<GameLeft>>();
        events.send(game1.into());
        events.send(game3.into());
        events.send(game4.into());
        app.update();

        // games 1, 3 are despawned, 2 and 4 remain in the world
        // 2 because no event was fired for it
        // 4 because it is not finished
        assert!(app.world().get::<Game>(game1).is_none());
        assert!(app.world().get::<Game>(game2).is_some());
        assert!(app.world().get::<Game>(game3).is_none());
        assert!(app.world().get::<Game>(game4).is_some());
    }

    /// Check that revert single action finds the action and sets the status to `NotConfirmed`;
    /// Check that status cannot be reverted for `Confirmed` actions;
    /// Check that revert all reverts all actions with status `WaitingConfirmation`.
    #[test]
    fn revert_pending_action_status() {
        let mut app = App::new();
        app.add_event::<ActionConfirmationFailed<u64>>();
        app.add_systems(Update, revert_action_status::<u64>);

        let initial_pending_actions = smallvec::smallvec![
            PendingAction::new(0, 0u64, ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, 1u64, ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, 2u64, ConfirmationStatus::WaitingConfirmation),
            PendingAction::new(0, 3u64, ConfirmationStatus::Confirmed),
            PendingAction::new(0, 4u64, ConfirmationStatus::Confirmed),
        ];

        let action_queue = app
            .world_mut()
            .spawn((
                PendingActionQueue::from(initial_pending_actions.clone()),
                ActiveGame,
            ))
            .id();

        // revert second action
        app.world_mut()
            .resource_mut::<Events<ActionConfirmationFailed<u64>>>()
            .send(ActionConfirmationFailed::revert_single(action_queue, 1u64));
        app.update();

        let initial_statuses_with_second_reverted = [
            ConfirmationStatus::WaitingConfirmation,
            ConfirmationStatus::NotConfirmed,
            ConfirmationStatus::WaitingConfirmation,
            ConfirmationStatus::Confirmed,
            ConfirmationStatus::Confirmed,
        ];
        // status changed to NotConfirmed
        itertools::assert_equal(
            get_component::<PendingActionQueue<u64>>(app.world(), action_queue)
                .iter()
                .map(|v| v.status()),
            initial_statuses_with_second_reverted,
        );

        // try to revert the last action
        app.world_mut()
            .resource_mut::<Events<ActionConfirmationFailed<u64>>>()
            .send(ActionConfirmationFailed::revert_single(action_queue, 4u64));
        app.update();

        // statuses hasn't been changed
        itertools::assert_equal(
            get_component::<PendingActionQueue<u64>>(app.world(), action_queue)
                .iter()
                .map(|v| v.status()),
            initial_statuses_with_second_reverted,
        );

        // revert all
        app.world_mut()
            .resource_mut::<Events<ActionConfirmationFailed<u64>>>()
            .send(ActionConfirmationFailed::revert_all(action_queue));
        app.update();
        itertools::assert_equal(
            get_component::<PendingActionQueue<u64>>(app.world(), action_queue)
                .iter()
                .map(|v| v.status()),
            [
                ConfirmationStatus::NotConfirmed,
                ConfirmationStatus::NotConfirmed,
                ConfirmationStatus::NotConfirmed,
                ConfirmationStatus::Confirmed,
                ConfirmationStatus::Confirmed,
            ],
        );
    }

    /// Check that SessionClosed creates ActionConfirmationFailed (all);
    /// Check that SessionActionSendFailed creates ActionConfirmationFailed (single);
    /// Check that SessionClosed for one entity and SessionActionSendFailed for another
    /// create ActionConfirmationFailed (all) and ActionConfirmationFailed (single) respectively;  
    /// Check that SessionClosed and SessionActionSendFailed for the same entity
    /// only create ActionConfirmationFailed (all).
    #[test]
    fn action_confirmation_failed_event() {
        let mut app = App::new();
        app.add_event::<grpc::SessionClosed>();
        app.add_event::<grpc::SessionActionSendFailed<u64>>();
        app.add_event::<ActionConfirmationFailed<u64>>();
        app.add_systems(Update, action_confirmation_failed::<u64>);

        let entity1 = app.world_mut().spawn_empty().id();
        let entity2 = app.world_mut().spawn_empty().id();

        app.world_mut()
            .resource_mut::<Events<grpc::SessionClosed>>()
            .send(entity1.into());
        app.update();

        let confirmation_failed_events = app
            .world()
            .resource::<Events<ActionConfirmationFailed<u64>>>();
        let mut cursor = confirmation_failed_events.get_cursor();
        let event = cursor.read(confirmation_failed_events).next().unwrap();
        assert_eq!(event.game(), entity1);
        assert_eq!(*event.revert_policy(), ActionStatusRevertPolicy::All);
        assert!(cursor.read(confirmation_failed_events).next().is_none());

        clear_events::<ActionConfirmationFailed<u64>>(app.world_mut());
        app.world_mut()
            .resource_mut::<Events<grpc::SessionActionSendFailed<u64>>>()
            .send(grpc::SessionActionSendFailed::new(entity1, 0));
        app.update();

        let confirmation_failed_events = app
            .world()
            .resource::<Events<ActionConfirmationFailed<u64>>>();
        let mut cursor = confirmation_failed_events.get_cursor();
        let event = cursor.read(confirmation_failed_events).next().unwrap();
        assert_eq!(event.game(), entity1);
        assert_eq!(*event.revert_policy(), ActionStatusRevertPolicy::Single(0));
        assert!(cursor.read(confirmation_failed_events).next().is_none());

        clear_events::<ActionConfirmationFailed<u64>>(app.world_mut());
        app.world_mut()
            .resource_mut::<Events<grpc::SessionActionSendFailed<u64>>>()
            .send(grpc::SessionActionSendFailed::new(entity1, 1));
        app.world_mut()
            .resource_mut::<Events<grpc::SessionClosed>>()
            .send(entity2.into());
        app.update();

        let confirmation_failed_events = app
            .world()
            .resource::<Events<ActionConfirmationFailed<u64>>>();
        let mut cursor = confirmation_failed_events.get_cursor();
        let event = cursor.read(confirmation_failed_events).next().unwrap();
        assert_eq!(event.game(), entity2);
        assert_eq!(*event.revert_policy(), ActionStatusRevertPolicy::All);
        let event = cursor.read(confirmation_failed_events).next().unwrap();
        assert_eq!(event.game(), entity1);
        assert_eq!(*event.revert_policy(), ActionStatusRevertPolicy::Single(1));
        assert!(cursor.read(confirmation_failed_events).next().is_none());

        clear_events::<ActionConfirmationFailed<u64>>(app.world_mut());
        app.world_mut()
            .resource_mut::<Events<grpc::SessionActionSendFailed<u64>>>()
            .send(grpc::SessionActionSendFailed::new(entity2, 1));
        app.world_mut()
            .resource_mut::<Events<grpc::SessionClosed>>()
            .send(entity2.into());
        app.update();

        let confirmation_failed_events = app
            .world()
            .resource::<Events<ActionConfirmationFailed<u64>>>();
        let mut cursor = confirmation_failed_events.get_cursor();
        let event = cursor.read(confirmation_failed_events).next().unwrap();
        assert_eq!(event.game(), entity2);
        assert_eq!(*event.revert_policy(), ActionStatusRevertPolicy::All);
        assert!(cursor.read(confirmation_failed_events).next().is_none());
    }

    /// Check:
    /// - empty queue, 1 enqueued
    /// - 1-action queue, 1 enqueued
    /// - empty queue, 1-action queue, 1 enqueued for both queues
    #[test]
    fn action_queue_next_changed_after_action_enqueue() {
        type ActionQueue = PendingActionQueue<u64>;

        let mut app = App::new();
        app.add_event::<ActionEnqueued<u64>>();
        app.add_event::<ActionDropped<u64>>();
        app.add_event::<ActionQueueNextChanged>();
        app.add_systems(Update, action_queue_next_changed::<u64>);

        let queue1 = app.world_mut().spawn(ActionQueue::default()).id();
        let queue2 = app.world_mut().spawn(ActionQueue::default()).id();

        // insert 1 action into the first queue and emit 1 event
        app.world_mut()
            .entity_mut(queue1)
            .get_mut::<ActionQueue>()
            .unwrap()
            .push(PendingAction::new(0, 0, ConfirmationStatus::NotConfirmed));
        app.world_mut()
            .resource_mut::<Events<ActionEnqueued<u64>>>()
            .send(ActionEnqueued::new(queue1, 0, 0));
        app.update();

        let events = app.world().resource::<Events<ActionQueueNextChanged>>();
        let mut cursor = events.get_cursor();
        assert_eq!(cursor.read(events).next().unwrap().get(), queue1);
        assert!(cursor.read(events).next().is_none());
        clear_events::<ActionQueueNextChanged>(app.world_mut());

        // insert 2 actions into the second queue and emit 1 event
        app.world_mut()
            .entity_mut(queue2)
            .insert(ActionQueue::from(SmallVec::from_elem(
                PendingAction::new(0, 0, ConfirmationStatus::NotConfirmed),
                2,
            )));
        app.world_mut()
            .resource_mut::<Events<ActionEnqueued<u64>>>()
            .send(ActionEnqueued::new(queue2, 0, 0));
        app.update();

        let events = app.world().resource::<Events<ActionQueueNextChanged>>();
        assert!(events.get_cursor().read(events).next().is_none());

        // first queue - 1 action, second - 2 actions, emit 1 event for both
        let mut events = app
            .world_mut()
            .resource_mut::<Events<ActionEnqueued<u64>>>();
        events.send(ActionEnqueued::new(queue1, 0, 0));
        events.send(ActionEnqueued::new(queue2, 0, 0));
        app.update();

        let events = app.world().resource::<Events<ActionQueueNextChanged>>();
        let mut cursor = events.get_cursor();
        assert_eq!(cursor.read(events).next().unwrap().get(), queue1);
        assert!(cursor.read(events).next().is_none());
    }

    /// Check:
    /// - drop from 1-action queue
    /// - drop from 2-action queue
    /// - drop from both 1-action and 2-action queue
    /// - 2 drops from one queue trigger 1 event
    #[test]
    fn action_queue_next_changed_after_action_drop() {
        type ActionQueue = PendingActionQueue<u64>;

        let mut app = App::new();
        app.add_event::<ActionEnqueued<u64>>();
        app.add_event::<ActionDropped<u64>>();
        app.add_event::<ActionQueueNextChanged>();
        app.add_systems(Update, action_queue_next_changed::<u64>);

        let queue1 = app.world_mut().spawn(ActionQueue::default()).id();
        let queue2 = app
            .world_mut()
            .spawn(ActionQueue::from(smallvec::smallvec![PendingAction::new(
                0,
                0,
                ConfirmationStatus::NotConfirmed
            )]))
            .id();

        // 1 action remains after drop
        app.world_mut()
            .resource_mut::<Events<ActionDropped<u64>>>()
            .send(ActionDropped::new(queue2, 0, 0, ""));
        app.update();

        let events = app.world().resource::<Events<ActionQueueNextChanged>>();
        let mut cursor = events.get_cursor();
        assert_eq!(cursor.read(events).next().unwrap().get(), queue2);
        assert!(cursor.read(events).next().is_none());
        clear_events::<ActionQueueNextChanged>(app.world_mut());

        // queue is empty after drop
        app.world_mut()
            .resource_mut::<Events<ActionDropped<u64>>>()
            .send(ActionDropped::new(queue1, 0, 0, ""));
        app.update();

        let events = app.world().resource::<Events<ActionQueueNextChanged>>();
        let mut cursor = events.get_cursor();
        assert_eq!(cursor.read(events).next().unwrap().get(), queue1);
        assert!(cursor.read(events).next().is_none());
        clear_events::<ActionQueueNextChanged>(app.world_mut());

        // one queue has 1 action and the other one is empty after drop
        let mut events = app.world_mut().resource_mut::<Events<ActionDropped<u64>>>();
        events.send(ActionDropped::new(queue1, 0, 0, ""));
        events.send(ActionDropped::new(queue2, 0, 0, ""));
        app.update();

        let events = app.world().resource::<Events<ActionQueueNextChanged>>();
        let mut cursor = events.get_cursor();
        assert_eq!(cursor.read(events).next().unwrap().get(), queue1);
        assert_eq!(cursor.read(events).next().unwrap().get(), queue2);
        assert!(cursor.read(events).next().is_none());
        clear_events::<ActionQueueNextChanged>(app.world_mut());

        // 2 drops from the queue trigger 1 event
        let mut events = app.world_mut().resource_mut::<Events<ActionDropped<u64>>>();
        events.send(ActionDropped::new(queue1, 0, 0, ""));
        events.send(ActionDropped::new(queue1, 0, 0, ""));
        app.update();

        let events = app.world().resource::<Events<ActionQueueNextChanged>>();
        let mut cursor = events.get_cursor();
        assert_eq!(cursor.read(events).next().unwrap().get(), queue1);
        assert!(cursor.read(events).next().is_none());
        clear_events::<ActionQueueNextChanged>(app.world_mut());
    }

    /// Check:
    /// - drop and enqueue in one queue trigger one event
    /// - the same as above but events order is changed
    #[test]
    fn action_queue_next_changed_after_action_enqueue_and_drop() {
        type ActionQueue = PendingActionQueue<u64>;

        let mut app = App::new();
        app.add_event::<ActionEnqueued<u64>>();
        app.add_event::<ActionDropped<u64>>();
        app.add_event::<ActionQueueNextChanged>();
        app.add_systems(Update, action_queue_next_changed::<u64>);

        let queue1 = app.world_mut().spawn(ActionQueue::default()).id();
        let queue2 = app
            .world_mut()
            .spawn(ActionQueue::from(SmallVec::from_elem(
                PendingAction::new(0, 0, ConfirmationStatus::NotConfirmed),
                2,
            )))
            .id();

        // drop and enqueue for queue2 should trigger one event
        app.world_mut()
            .resource_mut::<Events<ActionDropped<u64>>>()
            .send(ActionDropped::new(queue2, 0, 0, ""));
        app.world_mut()
            .resource_mut::<Events<ActionEnqueued<u64>>>()
            .send(ActionEnqueued::new(queue2, 0, 0));
        app.update();

        let events = app.world().resource::<Events<ActionQueueNextChanged>>();
        let mut cursor = events.get_cursor();
        assert_eq!(cursor.read(events).next().unwrap().get(), queue2);
        assert!(cursor.read(events).next().is_none());
        clear_events::<ActionQueueNextChanged>(app.world_mut());

        // enqueue and drop for queue2 should trigger one event
        app.world_mut()
            .resource_mut::<Events<ActionEnqueued<u64>>>()
            .send(ActionEnqueued::new(queue1, 0, 0));
        app.world_mut()
            .resource_mut::<Events<ActionDropped<u64>>>()
            .send(ActionDropped::new(queue1, 0, 0, ""));
        app.update();

        let events = app.world().resource::<Events<ActionQueueNextChanged>>();
        let mut cursor = events.get_cursor();
        assert_eq!(cursor.read(events).next().unwrap().get(), queue1);
        assert!(cursor.read(events).next().is_none());
        clear_events::<ActionQueueNextChanged>(app.world_mut());
    }
}
