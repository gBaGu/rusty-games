mod components;
mod error;
mod events;
mod resources;
mod systems;

use bevy::prelude::*;
use game_server::core;
use game_server::proto;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Endpoint, Uri};
use tonic_health::pb::health_client;

use components::{CallTask, ConnectClientTask, GameSession, ReceiveConnectionStatusTask};
use events::{AuthLinkReceived, AuthTokenReceived, LogInFailed};
use resources::{ConnectTimer, ConnectionStatusWatcher, ServerEndpoint, SessionCheckTimer};
use systems::*;

pub use components::LogInRequest;
pub use events::{
    CloseSession, Connected, Disconnected, LogInSuccess, LogOut, OpenSession, RpcResultReady,
    SessionActionReadyToSend, SessionActionSendFailed, SessionClosed, SessionErrorReceived,
    SessionOpened, SessionUpdateReceived,
};
pub use resources::GrpcClient;

pub const DEFAULT_GRPC_SERVER_ADDRESS: &str = "https://localhost:50051";
pub const CONNECT_INTERVAL_SEC: f32 = 5.0;
pub const GAME_SESSION_CHECK_INTERVAL_SEC: f32 = 1.0;
pub const GAME_SESSION_RECONNECT_INTERVAL_SEC: f32 = 1.0;
pub const HEALTH_RETRY_INTERVAL_SEC: f32 = 5.0;

pub type GameClient = proto::game_client::GameClient<Channel>;
pub type HealthClient = health_client::HealthClient<Channel>;
pub type AuthClient = proto::auth_client::AuthClient<Channel>;

pub fn client_exists_and_connected(client: Option<Res<GrpcClient>>) -> bool {
    matches!(client, Some(c) if c.connected())
}

type GrpcResult<T> = Result<T, error::GrpcError>;

#[derive(Debug)]
pub struct GameSessionUpdate<T> {
    player: core::PlayerPosition,
    action: T,
}

impl<T> GameSessionUpdate<T> {
    pub fn new(player: core::PlayerPosition, action: T) -> Self {
        Self { player, action }
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }

    pub fn action(&self) -> &T {
        &self.action
    }
}

/// System set for network communication systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkSystems;

pub struct GrpcPlugin {
    address: Uri,
    ca_cert: Certificate,
}

impl GrpcPlugin {
    pub fn new(ca_cert: &str) -> Self {
        let address = Uri::from_static(DEFAULT_GRPC_SERVER_ADDRESS);
        let ca_cert = Certificate::from_pem(ca_cert);
        Self { address, ca_cert }
    }
}

impl Plugin for GrpcPlugin {
    fn build(&self, app: &mut App) {
        let tls_cfg = ClientTlsConfig::new().ca_certificate(self.ca_cert.clone());
        let endpoint = Endpoint::from(self.address.clone())
            .tls_config(tls_cfg)
            .expect("unable to create tonic endpoint");
        app.configure_sets(Update, NetworkSystems.run_if(client_exists_and_connected))
            .init_resource::<ConnectTimer>()
            .init_resource::<SessionCheckTimer>()
            .insert_resource(ServerEndpoint::new(endpoint))
            .add_event::<Connected>()
            .add_event::<Disconnected>()
            .add_event::<RpcResultReady<proto::CreateGameReply>>()
            .add_event::<RpcResultReady<proto::MakeTurnReply>>()
            .add_event::<RpcResultReady<proto::GetGameReply>>()
            .add_event::<RpcResultReady<proto::GetPlayerGamesReply>>()
            .add_event::<OpenSession>()
            .add_event::<CloseSession>()
            .add_event::<SessionOpened>()
            .add_event::<SessionClosed>()
            .add_event::<SessionActionSendFailed<core::GridIndex>>()
            .add_event::<SessionActionReadyToSend<core::GridIndex>>()
            .add_event::<SessionUpdateReceived<core::GridIndex>>()
            .add_event::<SessionErrorReceived>()
            .add_event::<AuthLinkReceived>()
            .add_event::<AuthTokenReceived>()
            .add_event::<LogInSuccess>()
            .add_event::<LogInFailed>()
            .add_event::<LogOut>()
            .add_systems(
                Update,
                (
                    connect.run_if(
                        not(resource_exists::<GrpcClient>)
                            .and(not(any_with_component::<ConnectClientTask>)),
                    ),
                    handle_connect.run_if(any_with_component::<ConnectClientTask>),
                    receive_status.run_if(
                        resource_exists::<ConnectionStatusWatcher>
                            .and(not(any_with_component::<ReceiveConnectionStatusTask>)),
                    ),
                    handle_receive_status.run_if(any_with_component::<ReceiveConnectionStatusTask>),
                    handle_response::<proto::CreateGameReply>
                        .run_if(any_with_component::<CallTask<proto::CreateGameReply>>),
                    handle_response::<proto::MakeTurnReply>
                        .run_if(any_with_component::<CallTask<proto::MakeTurnReply>>),
                    handle_response::<proto::GetGameReply>
                        .run_if(any_with_component::<CallTask<proto::GetGameReply>>),
                    handle_response::<proto::GetPlayerGamesReply>
                        .run_if(any_with_component::<CallTask<proto::GetPlayerGamesReply>>),
                    init_open_session::<core::tic_tac_toe::TicTacToe>,
                    close_session::<core::tic_tac_toe::TicTacToe>,
                    session_closed::<core::tic_tac_toe::TicTacToe>,
                    delay_session_connection,
                    send_get_game_before_connect::<core::tic_tac_toe::TicTacToe>
                        .before(handle_response::<proto::GetGameReply>),
                    connect_session::<core::tic_tac_toe::TicTacToe>
                        .after(handle_response::<proto::GetGameReply>),
                    init_session_action_send_task::<core::tic_tac_toe::TicTacToe>,
                    init_session_update_receive_task::<core::tic_tac_toe::TicTacToe>,
                    handle_session_action_send::<core::GridIndex>,
                    handle_session_update_receive::<core::GridIndex>,
                    log_session_error,
                ),
            )
            .add_systems(
                Update,
                (
                    log_in_request,
                    open_auth_link,
                    store_token,
                    log_out,
                    handle_log_in_task,
                    receive_auth_link,
                    receive_auth_token,
                    log_log_in_error,
                ),
            );
    }
}
