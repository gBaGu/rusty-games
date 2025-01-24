use async_compat::CompatExt;
use bevy::prelude::*;
use bevy::tasks;
use bevy::tasks::futures_lite::future;
use bevy::tasks::IoTaskPool;
use game_server::core::ToProtobuf as _;
use game_server::{core, proto};
use tonic::transport;

use super::components::{
    CallTask, ConnectClientTask, ConnectingGameSession, ReceiveConnectionStatusTask,
    ReceiveSessionUpdateTask, ReconnectSessionBundle, ReconnectSessionTimer, SendActionTask,
};
use super::events::{RpcResultReady, SessionErrorReceived, SessionUpdateReceived};
use super::resources::{ConnectTimer, ConnectionStatusWatcher, SessionCheckTimer};
use super::{
    CloseSession, Connected, Disconnected, GameClient, GameSession, GrpcClient, HealthClient,
    OpenSession, SessionActionReadyToSend, SessionActionSendFailed, SessionClosed, SessionOpened,
    DEFAULT_GRPC_SERVER_ADDRESS,
};
use crate::common::PollOnce;
use crate::game::{ActiveGame, NetworkGame};
use crate::Settings;

pub fn connect(mut commands: Commands, mut timer: ResMut<ConnectTimer>, time: Res<Time>) {
    if timer.tick(time.delta()).just_finished() {
        debug!("trying to connect to grpc server...");
        let task = IoTaskPool::get().spawn(async move {
            transport::Endpoint::new(DEFAULT_GRPC_SERVER_ADDRESS)?
                .connect()
                .compat()
                .await
        });
        commands.spawn(ConnectClientTask::from(task));
    }
}

pub fn handle_connect(
    mut commands: Commands,
    mut connect_task: Query<(Entity, &mut ConnectClientTask)>,
    mut connected: EventWriter<Connected>,
    client: Option<Res<GrpcClient>>,
) {
    let Ok((entity, mut task)) = connect_task.get_single_mut() else {
        error!("multiple connect tasks present");
        return;
    };
    if let Some(res) = task.poll_once(commands.entity(entity)) {
        if client.is_some() {
            return;
        }
        match res {
            Ok(c) => {
                let client = GrpcClient::new(GameClient::new(c.clone()));
                debug!("server connection established, creating health watcher");
                let watcher = ConnectionStatusWatcher::start(HealthClient::new(c));
                commands.insert_resource(client);
                commands.insert_resource(watcher);
                connected.send(Connected);
            }
            Err(err) => {
                debug!("grpc client connect failed: {:?}", err);
            }
        }
    }
}

pub fn receive_status(mut commands: Commands, watcher: Res<ConnectionStatusWatcher>) {
    let receiver = watcher.update_receiver();
    if !receiver.is_closed() {
        let task = IoTaskPool::get().spawn(async move { receiver.recv().await });
        commands.spawn(ReceiveConnectionStatusTask::from(task));
    } else {
        debug!("ConnectStatusWatcher is finished");
        commands.remove_resource::<ConnectionStatusWatcher>();
    }
}

pub fn handle_receive_status(
    mut commands: Commands,
    mut receive_update_task: Query<(Entity, &mut ReceiveConnectionStatusTask)>,
    mut connected: EventWriter<Connected>,
    mut disconnected: EventWriter<Disconnected>,
    client: Option<ResMut<GrpcClient>>,
) {
    let Ok((entity, mut task)) = receive_update_task.get_single_mut() else {
        error!("multiple receive connection status tasks present");
        return;
    };
    if let Some(res) = task.poll_once(commands.entity(entity)) {
        if let Some(mut client) = client {
            let updated_status = res.unwrap_or_else(|err| {
                error!("failed to get connection status: {}", err);
                false
            });
            if client.connected() && !updated_status {
                info!("grpc client disconnected");
                disconnected.send(Disconnected);
            } else if !client.connected() && updated_status {
                info!("grpc client connected");
                connected.send(Connected);
            }
            client.set_connected(updated_status);
        }
    }
}

pub fn handle_response<T: Send + Sync + 'static>(
    mut commands: Commands,
    mut task: Query<(Entity, &mut CallTask<T>)>,
    mut response_ready: EventWriter<RpcResultReady<T>>,
) {
    for (entity, mut task) in task.iter_mut() {
        if let Some(res) = task.poll_once(commands.entity(entity)) {
            response_ready.send(RpcResultReady::new(entity, res));
        }
    }
}

/// Listen to [`CloseSession`] event and close input channel of the [`GameSession`].
pub fn close_session<T>(
    session: Query<&GameSession<T, T::TurnData>>,
    mut close_session: EventReader<CloseSession>,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Send,
{
    for event in close_session.read() {
        if let Ok(session) = session.get(**event) {
            session.action_sender().close();
        }
    }
}

/// Polls unfinished session tasks and if the task is ready remove [`GameSession`] from the entity
/// and send [`SessionClosed`] event.
pub fn session_closed<T>(
    mut commands: Commands,
    mut session: Query<(
        Entity,
        &mut GameSession<T, T::TurnData>,
        Option<&ActiveGame>,
    )>,
    mut timer: ResMut<SessionCheckTimer>,
    mut session_closed: EventWriter<SessionClosed>,
    mut open_session: EventWriter<OpenSession>,
    time: Res<Time>,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Send,
{
    if timer.tick(time.delta()).just_finished() {
        for (session_entity, mut session, active) in session.iter_mut() {
            if tasks::block_on(future::poll_once(session.task_mut())).is_some() {
                commands
                    .entity(session_entity)
                    .remove::<GameSession<T, T::TurnData>>();
                session_closed.send(SessionClosed::new(session_entity));
                if active.is_some() {
                    open_session.send(OpenSession::new_delayed(session_entity));
                }
            }
        }
    }
}

/// Receive the [`OpenSession`] event and insert components required for session initialization
/// into the entity contained in the event.
/// `T` is a game type.
pub fn init_open_session<T>(mut commands: Commands, mut open_session: EventReader<OpenSession>)
where
    T: Send + Sync + 'static,
{
    for event in open_session.read() {
        let mut session_cmds = commands.entity(event.game());
        if event.delayed() {
            session_cmds.insert(ReconnectSessionBundle::<T>::default());
        } else {
            session_cmds.insert(ConnectingGameSession::<T>::default());
        }
    }
}

/// Update [`ReconnectSessionTimer`] component and remove it once finished.
pub fn delay_session_connection(
    mut commands: Commands,
    mut timer: Query<(Entity, &mut ReconnectSessionTimer)>,
    time: Res<Time>,
) {
    for (session_entity, mut timer) in timer.iter_mut() {
        if timer.tick(time.delta()).just_finished() {
            commands
                .entity(session_entity)
                .remove::<ReconnectSessionTimer>();
        }
    }
}

/// If session entity doesn't have [`ReconnectSessionTimer`] component send `GetGame` request.
pub fn send_get_game_before_connect<T>(
    mut commands: Commands,
    mut connecting_session: Query<
        (Entity, &NetworkGame),
        (
            With<ConnectingGameSession<T>>,
            Without<ReconnectSessionTimer>,
            Without<CallTask<proto::GetGameReply>>,
        ),
    >,
    client: Option<Res<GrpcClient>>,
) where
    T: proto::GetGameType + Send + Sync + 'static,
{
    for (game_entity, network_game) in connecting_session.iter_mut() {
        let Some(ref client) = client else {
            error!("unable to reconnect session: grpc client is not connected");
            commands
                .entity(game_entity)
                .remove::<ConnectingGameSession<T>>();
            continue;
        };
        match client.get_game::<T>(**network_game) {
            Ok(task) => {
                commands.entity(game_entity).insert(task);
            }
            Err(err) => {
                error!("unable to reconnect session: GetGame call failed: {}", err);
                commands
                    .entity(game_entity)
                    .remove::<ConnectingGameSession<T>>();
            }
        }
    }
}

/// Receive `GetGameReply` and if session entity doesn't have [`ReconnectSessionTimer`] component
/// start game session by sending grpc request.
/// On success insert game session component into the game entity and send [`SessionOpened`] event.
pub fn connect_session<T>(
    mut commands: Commands,
    connecting_session: Query<
        &NetworkGame,
        (
            With<ConnectingGameSession<T>>,
            Without<ReconnectSessionTimer>,
        ),
    >,
    mut get_game_reply: EventReader<RpcResultReady<proto::GetGameReply>>,
    mut session_opened: EventWriter<SessionOpened>,
    client: Option<Res<GrpcClient>>,
    settings: Res<Settings>,
) where
    T: core::Game + proto::GetGameType + Send + Sync + 'static,
    T::TurnData: Send + 'static,
{
    for event in get_game_reply.read() {
        let Ok(network_game) = connecting_session.get(event.entity()) else {
            continue;
        };
        commands
            .entity(event.entity())
            .remove::<ConnectingGameSession<T>>();
        let Some(user) = settings.user_id() else {
            error!("unable to connect session: user is not logged in");
            continue;
        };
        let Some(ref client) = client else {
            error!("unable to connect session: grpc client is not connected");
            continue;
        };
        match event.result() {
            Ok(_) => match client.game_session::<T>(**network_game, user) {
                Ok(session) => {
                    commands.entity(event.entity()).insert(session);
                    session_opened.send(SessionOpened::new(event.entity()));
                }
                Err(err) => error!(
                    "unable to connect session: GameSession call failed: {}",
                    err
                ),
            },
            Err(err) => {
                error!("unable to connect session: GetGame call failed: {}", err);
            }
        }
    }
}

/// Receive the [`SessionActionReadyToSend`] event, find [`GameSession`] entity and
/// create a task that will send encoded action from event into session sender.
/// Send the [`SessionActionSendFailed`] event if there is no [`GameSession`] entity,
/// another [`SendActionTask`] is present or action encoding failed.
pub fn init_session_action_send_task<T>(
    mut commands: Commands,
    session: Query<(&GameSession<T, T::TurnData>, Option<&SendActionTask>)>,
    mut action_ready: EventReader<SessionActionReadyToSend<T::TurnData>>,
    mut action_send_failed: EventWriter<SessionActionSendFailed<T::TurnData>>,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Copy + Send + Sync + 'static,
{
    for event in action_ready.read() {
        let session_entity = event.session_entity();
        let action = *event.action();
        let Ok((session, send_action_task)) = session.get(session_entity) else {
            error!("failed to send session action: session component not found");
            action_send_failed.send(SessionActionSendFailed::new(session_entity, action));
            continue;
        };
        if send_action_task.is_some() {
            action_send_failed.send(SessionActionSendFailed::new(session_entity, action));
            continue;
        }
        let action_data = match action.to_protobuf() {
            Ok(data) => data,
            Err(_err) => {
                action_send_failed.send(SessionActionSendFailed::new(session_entity, action));
                continue;
            }
        };
        let sender = session.action_sender();
        let task = IoTaskPool::get().spawn(async move { sender.send(action_data).await });
        commands
            .entity(session_entity)
            .insert(SendActionTask::from(task));
    }
}

/// Poll channel send task. If it returns with error, send the [`SessionActionSendFailed`] event.  
/// `T` is a type of action.
pub fn handle_session_action_send<T>(
    mut commands: Commands,
    mut task: Query<(Entity, &mut SendActionTask)>,
    mut action_send_failed: EventWriter<SessionActionSendFailed<T>>,
) where
    T: core::FromProtobuf + Send + Sync + 'static,
{
    for (task_entity, mut task) in task.iter_mut() {
        if let Some(res) = task.poll_once(commands.entity(task_entity)) {
            if let Err(err) = res {
                error!("send session action task failed: {}", err);
                match T::from_protobuf(&err.into_inner()) {
                    Ok(action) => {
                        action_send_failed.send(SessionActionSendFailed::new(task_entity, action));
                    }
                    Err(err) => {
                        error!("cannot decode action back from bytes: {}", err);
                    }
                }
            }
        }
    }
}

/// Find [`GameSession`] entity that has no [`ReceiveSessionUpdateTask`] component and
/// create a task that will receive update from session update receiver.
pub fn init_session_update_receive_task<T>(
    mut commands: Commands,
    session: Query<
        (Entity, &GameSession<T, T::TurnData>),
        Without<ReceiveSessionUpdateTask<T::TurnData>>,
    >,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Send,
{
    for (session_entity, session) in session.iter() {
        let receiver = session.update_receiver();
        if !receiver.is_closed() {
            trace!("create receive session update task");
            let task = IoTaskPool::get().spawn(async move { receiver.recv().await });
            commands
                .entity(session_entity)
                .insert(ReceiveSessionUpdateTask::from(task));
        }
    }
}

/// Poll channel receive task. If it returned with error, just print a message, otherwise
/// send [`SessionUpdateReceived`] event in case of successful update or
/// [`SessionErrorReceived`] event in case of session error.  
/// `T` is a type of action.
pub fn handle_session_update_receive<T>(
    mut commands: Commands,
    mut session: Query<(Entity, &mut ReceiveSessionUpdateTask<T>)>,
    mut update_received: EventWriter<SessionUpdateReceived<T>>,
    mut error_received: EventWriter<SessionErrorReceived>,
) where
    T: Copy + Send + Sync + 'static,
{
    for (session_entity, mut task) in session.iter_mut() {
        if let Some(res) = task.poll_once(commands.entity(session_entity)) {
            match res {
                Ok(Ok(update)) => {
                    update_received.send(SessionUpdateReceived::<T>::new(
                        session_entity,
                        update.player(),
                        *update.action(),
                    ));
                }
                Ok(Err(err)) => {
                    error_received.send(SessionErrorReceived::new(session_entity, err));
                }
                Err(err) => {
                    // channel is closed, print and do nothing
                    warn!("failed to read from session update channel: {}", err);
                }
            };
        }
    }
}

/// Print game session errors.
pub fn log_session_error(mut error_received: EventReader<SessionErrorReceived>) {
    for event in error_received.read() {
        error!(
            "game session ({}) error received: {}",
            event.session_entity(),
            event.error(),
        );
    }
}

#[cfg(test)]
mod test {
    use bevy::tasks::TaskPool;

    use super::*;
    use crate::grpc::error::GrpcError;
    use crate::grpc::GameSessionUpdate;

    type DummySession = GameSession<DummyGame, ()>;

    struct DummyGame {
        players: core::PlayerIdQueue<core::PlayerPosition>,
        board: core::Grid<(), typenum::U0, typenum::U0>,
    }

    impl core::Game for DummyGame {
        const NUM_PLAYERS: u8 = 1;
        type TurnData = ();
        type Players = core::PlayerIdQueue<core::PlayerPosition>;
        type Board = core::Grid<(), typenum::U0, typenum::U0>;

        fn new() -> Self {
            Self {
                players: Self::Players::new(vec![0]),
                board: Self::Board::default(),
            }
        }

        fn update(
            &mut self,
            _id: core::PlayerPosition,
            _data: Self::TurnData,
        ) -> core::GameResult<core::GameState> {
            Ok(core::GameState::Turn(0))
        }

        fn board(&self) -> &Self::Board {
            &self.board
        }

        fn board_mut(&mut self) -> &mut Self::Board {
            &mut self.board
        }

        fn set_board(&mut self, board: Self::Board) {
            self.board = board;
        }

        fn players(&self) -> &Self::Players {
            &self.players
        }

        fn players_mut(&mut self) -> &mut Self::Players {
            &mut self.players
        }

        fn state(&self) -> core::GameState {
            core::GameState::Turn(0)
        }

        fn set_state(&mut self, _state: core::GameState) {}
    }

    fn send_action_ready_to_send<T>(world: &mut World, session: Entity, action: T)
    where
        T: Send + Sync + 'static,
    {
        world
            .resource_mut::<Events<SessionActionReadyToSend<T>>>()
            .send(SessionActionReadyToSend::new(session, action));
    }

    /// Send [`OpenSession`] event and check that all required components were inserted.
    #[test]
    fn start_session_initialization() {
        let mut app = App::new();
        app.add_event::<OpenSession>();
        app.add_systems(Update, init_open_session::<DummyGame>);

        let session1 = app.world_mut().spawn_empty().id();
        let session2 = app.world_mut().spawn_empty().id();

        app.world_mut()
            .resource_mut::<Events<OpenSession>>()
            .send(OpenSession::new(session1));
        app.world_mut()
            .resource_mut::<Events<OpenSession>>()
            .send(OpenSession::new_delayed(session2));
        // this should insert ConnectingGameSession into both sessions and ReconnectSessionTimer into session2
        app.update();
        assert!(app
            .world()
            .entity(session1)
            .contains::<ConnectingGameSession<DummyGame>>());
        assert!(app
            .world()
            .entity(session2)
            .contains::<ConnectingGameSession<DummyGame>>());
        assert!(app
            .world()
            .entity(session2)
            .contains::<ReconnectSessionTimer>());
    }

    /// Spawn two session entities: one with [`ActiveGame`] component and one without;
    /// trigger [`CloseSession`].  
    /// Check that [`SessionClosed`] event is triggered for both entities.  
    /// Check that for the session without [`ActiveGame`] component
    /// [`OpenSession`] event is triggered after session is closed.
    #[test]
    fn session_close() {
        IoTaskPool::get_or_init(|| TaskPool::default());
        let mut app = App::new();
        let mut timer = SessionCheckTimer::default();
        timer.pause();
        app.init_resource::<Time>();
        app.insert_resource(timer);
        app.add_event::<CloseSession>();
        app.add_event::<SessionClosed>();
        app.add_event::<OpenSession>();
        app.add_systems(
            Update,
            (close_session::<DummyGame>, session_closed::<DummyGame>),
        );

        let (s_action, r_action) = async_channel::unbounded();
        let (s_update, r_update) = async_channel::unbounded();
        let session_active = app
            .world_mut()
            .spawn((
                DummySession::new(
                    IoTaskPool::get().spawn(async move {
                        let _s_update = s_update; // move it here
                        while let Ok(_) = r_action.recv().await {}
                        info!("active session is closed");
                    }),
                    s_action,
                    r_update,
                ),
                ActiveGame,
            ))
            .id();
        let (s_action, r_action) = async_channel::unbounded();
        let (s_update, r_update) = async_channel::unbounded();
        let session_inactive = app
            .world_mut()
            .spawn(DummySession::new(
                IoTaskPool::get().spawn(async move {
                    let _s_update = s_update; // move it here
                    while let Ok(_) = r_action.recv().await {}
                    info!("inactive session is closed");
                }),
                s_action,
                r_update,
            ))
            .id();

        app.world_mut()
            .resource_mut::<Events<CloseSession>>()
            .send(CloseSession::new(session_active));
        app.world_mut()
            .resource_mut::<Events<CloseSession>>()
            .send(CloseSession::new(session_inactive));
        // this should just close action sender for each session
        app.update();
        assert!(app
            .world()
            .entity(session_active)
            .contains::<DummySession>());
        assert!(app
            .world()
            .entity(session_inactive)
            .contains::<DummySession>());

        while !(app
            .world()
            .entity(session_active)
            .get::<DummySession>()
            .unwrap()
            .update_receiver()
            .is_closed()
            && app
                .world()
                .entity(session_inactive)
                .get::<DummySession>()
                .unwrap()
                .update_receiver()
                .is_closed())
        {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        let mut timer = app.world_mut().resource_mut::<SessionCheckTimer>();
        let duration = timer.duration();
        timer.set_elapsed(duration);
        timer.unpause();
        // this should remove game session component and trigger SessionClosed
        app.update();
        assert!(!app
            .world()
            .entity(session_active)
            .contains::<DummySession>());
        assert!(!app
            .world()
            .entity(session_inactive)
            .contains::<DummySession>());
        let session_closed_events = app.world().resource::<Events<SessionClosed>>();
        let mut cursor = session_closed_events.get_cursor();
        assert_eq!(
            **cursor.read(session_closed_events).next().unwrap(),
            session_active
        );
        assert_eq!(
            **cursor.read(session_closed_events).next().unwrap(),
            session_inactive
        );
        let open_session_events = app.world().resource::<Events<OpenSession>>();
        let mut cursor = open_session_events.get_cursor();
        assert_eq!(
            cursor.read(open_session_events).next().unwrap().game(),
            session_active
        );
    }

    /// Initialize action send by SessionActionReadyToSend event.
    /// Check that send task is deleted after [`handle_session_action_send`].
    /// Check that [`SessionActionSendFailed`] event is triggered when:  
    ///  - [`GameSession`] component is missing;  
    ///  - [`SendActionTask`] component is present;  
    ///  - the actions channel is closed.
    #[test]
    fn session_send_action() {
        type SendFailedEvents = Events<SessionActionSendFailed<()>>;

        IoTaskPool::get_or_init(|| TaskPool::default());
        let mut app = App::new();
        app.add_event::<SessionActionReadyToSend<()>>();
        app.add_event::<SessionActionSendFailed<()>>();
        app.add_systems(
            Update,
            (
                // handle before init so the send task stay after update
                init_session_action_send_task::<DummyGame>.after(handle_session_action_send::<()>),
                handle_session_action_send::<()>,
            ),
        );

        let session = app.world_mut().spawn_empty().id();
        send_action_ready_to_send(app.world_mut(), session, ());

        // this should trigger send error because there's no session component
        app.update();
        let err_events = app.world().resource::<SendFailedEvents>();
        let mut cursor = err_events.get_cursor();
        assert_eq!(
            cursor.read(err_events).next().unwrap().session_entity(),
            session
        ); // got error event
        assert!(cursor.read(err_events).next().is_none());
        // ensure old events are dropped
        app.world_mut().resource_mut::<SendFailedEvents>().clear();

        let (s, r) = async_channel::unbounded();
        let (notify_sender, notify_receiver) = async_channel::unbounded();
        let r_cloned = r.clone();
        app.world_mut()
            .entity_mut(session)
            .insert(DummySession::new(
                IoTaskPool::get().spawn(async move {
                    while let Ok(_) = r_cloned.recv().await {
                        info!("action is sent!");
                        notify_sender.send(()).await.expect("notification failed");
                    }
                }),
                s,
                async_channel::unbounded().1,
            ));

        // insert send action task and check that another one cannot be created
        let pending_task: SendActionTask = IoTaskPool::get().spawn(future::pending()).into();
        app.world_mut().entity_mut(session).insert(pending_task);
        send_action_ready_to_send(app.world_mut(), session, ());
        // this should trigger send error because previous SendActionTask isn't completed
        app.update();
        let err_events = app.world().resource::<SendFailedEvents>();
        let mut cursor = err_events.get_cursor();
        assert_eq!(
            cursor.read(err_events).next().unwrap().session_entity(),
            session
        ); // got error event
        assert!(cursor.read(err_events).next().is_none());
        // remove infinite task
        app.world_mut()
            .entity_mut(session)
            .remove::<SendActionTask>();

        send_action_ready_to_send(app.world_mut(), session, ());
        // this should create send task
        app.update();
        assert!(app.world().entity(session).contains::<SendActionTask>());

        // this is needed in order to ensure that the send task has completed
        tasks::block_on(IoTaskPool::get().spawn(async move {
            notify_receiver.recv().await.unwrap();
        }));
        // this should remove send task
        app.update();
        assert!(!app.world().entity(session).contains::<SendActionTask>());

        r.close();
        send_action_ready_to_send(app.world_mut(), session, ());
        // this should create send task
        app.update();
        assert!(app.world().entity(session).contains::<SendActionTask>());

        // this should remove send task and trigger error event because the channel is closed
        app.update();
        assert!(!app.world().entity(session).contains::<SendActionTask>());
        let err_events = app.world().resource::<SendFailedEvents>();
        let mut cursor = err_events.get_cursor();
        assert_eq!(
            cursor.read(err_events).next().unwrap().session_entity(),
            session
        ); // got error event
        assert!(cursor.read(err_events).next().is_none());
    }

    /// Spawn game session with receiver that will receive some predefined updates.
    /// Check that all updates are received and in the end one more receive task remains waiting.
    #[test]
    fn session_receive_update() {
        type ReceiveUpdateTask = ReceiveSessionUpdateTask<()>;

        IoTaskPool::get_or_init(|| TaskPool::default());
        let mut app = App::new();
        app.add_event::<SessionUpdateReceived<()>>();
        app.add_event::<SessionErrorReceived>();
        app.add_systems(
            Update,
            (
                init_session_update_receive_task::<DummyGame>
                    .before(handle_session_update_receive::<()>),
                handle_session_update_receive::<()>,
            ),
        );

        let (s, r) = async_channel::unbounded();
        let session = app
            .world_mut()
            .spawn(DummySession::new(
                IoTaskPool::get().spawn(future::ready(())),
                async_channel::unbounded().0,
                r,
            ))
            .id();

        // this should spawn receive task
        app.update();
        assert!(app.world().entity(session).contains::<ReceiveUpdateTask>());

        let s_cloned = s.clone();
        tasks::block_on(IoTaskPool::get().spawn(async move {
            s_cloned
                .send(Ok(GameSessionUpdate::new(0, ())))
                .await
                .unwrap();
        }));

        // this should trigger session update event and remove receive task
        app.update();
        assert!(!app.world().entity(session).contains::<ReceiveUpdateTask>());
        let update_events = app.world().resource::<Events<SessionUpdateReceived<()>>>();
        let err_events = app.world().resource::<Events<SessionErrorReceived>>();
        let mut cursor = update_events.get_cursor();
        assert!(cursor.read(update_events).next().is_some()); // got update event
        assert!(cursor.read(update_events).next().is_none());
        assert!(err_events.get_cursor().read(err_events).next().is_none());

        // this should spawn receive task
        app.update();
        assert!(app.world().entity(session).contains::<ReceiveUpdateTask>());

        let s_cloned = s.clone();
        tasks::block_on(IoTaskPool::get().spawn(async move {
            s_cloned
                .send(Ok(GameSessionUpdate::new(0, ())))
                .await
                .unwrap();
        }));

        // this should trigger session update event and remove receive task
        app.update();
        assert!(!app.world().entity(session).contains::<ReceiveUpdateTask>());
        let update_events = app.world().resource::<Events<SessionUpdateReceived<()>>>();
        let err_events = app.world().resource::<Events<SessionErrorReceived>>();
        let mut cursor = update_events.get_cursor();
        assert!(cursor.read(update_events).next().is_some()); // got update event
        assert!(cursor.read(update_events).next().is_none());
        assert!(err_events.get_cursor().read(err_events).next().is_none());

        // this should spawn receive task
        app.update();
        assert!(app.world().entity(session).contains::<ReceiveUpdateTask>());

        let s_cloned = s.clone();
        tasks::block_on(IoTaskPool::get().spawn(async move {
            s_cloned
                .send(Err(GrpcError::GameSessionUpdateFailed("".into())))
                .await
                .unwrap();
        }));

        // this should trigger session error event and remove receive task
        app.update();
        assert!(!app.world().entity(session).contains::<ReceiveUpdateTask>());
        let update_events = app.world().resource::<Events<SessionUpdateReceived<()>>>();
        let err_events = app.world().resource::<Events<SessionErrorReceived>>();
        assert!(update_events
            .get_cursor()
            .read(update_events)
            .next()
            .is_none());
        let mut cursor = err_events.get_cursor();
        assert!(cursor.read(err_events).next().is_some()); // got error event
        assert!(cursor.read(err_events).next().is_none());

        // this should spawn receive task
        app.update();
        assert!(app.world().entity(session).contains::<ReceiveUpdateTask>());

        // this should do nothing
        app.update();
        assert!(app.world().entity(session).contains::<ReceiveUpdateTask>());
        let update_events = app.world().resource::<Events<SessionUpdateReceived<()>>>();
        let err_events = app.world().resource::<Events<SessionErrorReceived>>();
        assert!(update_events
            .get_cursor()
            .read(update_events)
            .next()
            .is_none());
        assert!(err_events.get_cursor().read(err_events).next().is_none());

        s.close();
        // this should just print a message
        app.update();
        let update_events = app.world().resource::<Events<SessionUpdateReceived<()>>>();
        let err_events = app.world().resource::<Events<SessionErrorReceived>>();
        assert!(update_events
            .get_cursor()
            .read(update_events)
            .next()
            .is_none());
        assert!(err_events.get_cursor().read(err_events).next().is_none());
    }
}
