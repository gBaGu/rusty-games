use std::future::Future;
use std::pin::Pin;
use tokio::task::JoinError;

use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status, Streaming};

use super::RpcResult;
use super::error::RpcError;
use super::lobby_manager::LobbyManager;
use crate::core::chess::Chess;
use crate::core::tic_tac_toe::TicTacToe;
use crate::proto;

pub type GameId = u64;
pub type UserId = u64;

pub type RpcInnerResult<T> = Result<T, RpcError>;

#[derive(Default)]
pub struct GameImpl {
    tic_tac_toe: LobbyManager<TicTacToe>,
    chess: LobbyManager<Chess>,
}

impl GameImpl {
    pub fn start_workers(
        &mut self,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<(), JoinError>> {
        let ttt_worker = self.tic_tac_toe.start_worker(ct.clone());
        let chess_worker = self.chess.start_worker(ct.clone());
        async move {
            ttt_worker.await?;
            chess_worker.await
        }
    }
}

#[tonic::async_trait]
impl proto::game_server::Game for GameImpl {
    async fn create_game(
        &self,
        request: Request<proto::CreateGameRequest>,
    ) -> RpcResult<proto::CreateGameReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type =
            proto::GameType::try_from(request.game_type).map_err(|_| RpcError::InvalidGameType)?;
        let player1 = *request
            .player_ids
            .first()
            .ok_or(RpcError::RequestDataMissing("player_ids".into()))?;
        let game_info = match game_type {
            proto::GameType::TicTacToe => self.tic_tac_toe.create(player1, &request.player_ids)?,
            proto::GameType::Chess => self.chess.create(player1, &request.player_ids)?,
            proto::GameType::Unspecified => return Err(RpcError::InvalidGameType.into()),
        };
        Ok(Response::new(proto::CreateGameReply {
            game_info: Some(game_info),
        }))
    }

    async fn make_turn(
        &self,
        request: Request<proto::MakeTurnRequest>,
    ) -> RpcResult<proto::MakeTurnReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type =
            proto::GameType::try_from(request.game_type).map_err(|_| RpcError::InvalidGameType)?;
        let game = request.game_id;
        let player = request.player_id;
        let game_state = match game_type {
            proto::GameType::TicTacToe => {
                self.tic_tac_toe.update(game, player, &request.turn_data)?
            }
            proto::GameType::Chess => self.chess.update(game, player, &request.turn_data)?,
            proto::GameType::Unspecified => return Err(RpcError::InvalidGameType.into()),
        };
        Ok(Response::new(proto::MakeTurnReply {
            game_state: Some(game_state.into()),
        }))
    }

    type GameSessionStream =
        Pin<Box<dyn Stream<Item = Result<proto::GameSessionReply, Status>> + Send + 'static>>;

    async fn game_session(
        &self,
        request: Request<Streaming<proto::GameSessionRequest>>,
    ) -> RpcResult<Self::GameSessionStream> {
        println!("Got GameSession request");

        let mut input_stream = request.into_inner();
        let Some(request) = input_stream.next().await else {
            // got empty stream, return empty
            return Ok(Response::new(Box::pin(tokio_stream::empty())));
        };
        let request = request?.request.ok_or(RpcError::EmptyRequest)?;
        let proto::game_session_request::Request::Init(session) = request else {
            return Err(RpcError::unexpected_request("Init", request.name()).into());
        };
        let game_type =
            proto::GameType::try_from(session.game_type).map_err(|_| RpcError::InvalidGameType)?;
        let game = session.game_id;
        let player = session.player_id;
        let stream = match game_type {
            proto::GameType::TicTacToe => {
                self.tic_tac_toe
                    .start_game_session(game, player, input_stream)?
            }
            proto::GameType::Chess => self.chess.start_game_session(game, player, input_stream)?,
            proto::GameType::Unspecified => return Err(RpcError::InvalidGameType.into()),
        };
        Ok(Response::new(stream))
    }

    async fn delete_game(
        &self,
        request: Request<proto::DeleteGameRequest>,
    ) -> RpcResult<proto::DeleteGameReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type =
            proto::GameType::try_from(request.game_type).map_err(|_| RpcError::InvalidGameType)?;
        // For now, it's a creator id
        let game = request.game_id;
        match game_type {
            proto::GameType::TicTacToe => self.tic_tac_toe.delete(game)?,
            proto::GameType::Chess => self.chess.delete(game)?,
            proto::GameType::Unspecified => return Err(RpcError::InvalidGameType.into()),
        };
        Ok(Response::new(proto::DeleteGameReply {}))
    }

    async fn get_game(
        &self,
        request: Request<proto::GetGameRequest>,
    ) -> RpcResult<proto::GetGameReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type =
            proto::GameType::try_from(request.game_type).map_err(|_| RpcError::InvalidGameType)?;
        let game = request.game_id;
        let info = match game_type {
            proto::GameType::TicTacToe => self.tic_tac_toe.get_game(game)?,
            proto::GameType::Chess => self.chess.get_game(game)?,
            proto::GameType::Unspecified => return Err(RpcError::InvalidGameType.into()),
        };
        Ok(Response::new(proto::GetGameReply {
            game_info: Some(info),
        }))
    }

    async fn get_player_games(
        &self,
        request: Request<proto::GetPlayerGamesRequest>,
    ) -> RpcResult<proto::GetPlayerGamesReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type =
            proto::GameType::try_from(request.game_type).map_err(|_| RpcError::InvalidGameType)?;
        let player = request.player_id;
        let games = match game_type {
            proto::GameType::TicTacToe => self.tic_tac_toe.get_player_games(player)?,
            proto::GameType::Chess => self.chess.get_player_games(player)?,
            proto::GameType::Unspecified => return Err(RpcError::InvalidGameType.into()),
        };
        Ok(Response::new(proto::GetPlayerGamesReply { games }))
    }
}
