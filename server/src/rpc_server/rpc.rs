use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::proto::{
    game_server::Game, CreateGameReply, CreateGameRequest, DeleteGameReply, DeleteGameRequest,
    GameType, GetPlayerGamesReply, GetPlayerGamesRequest, MakeTurnReply, MakeTurnRequest,
};
use crate::rpc_server::game_storage::GameStorage;

pub type RpcResult<T> = Result<Response<T>, Status>;

#[derive(Debug, Default)]
pub struct GameImpl {
    games: Arc<GameStorage>,
}

#[tonic::async_trait]
impl Game for GameImpl {
    async fn create_game(&self, request: Request<CreateGameRequest>) -> RpcResult<CreateGameReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type = GameType::try_from(request.game_type)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        let player1 = request
            .player_ids
            .first()
            .cloned()
            .ok_or_else(|| Status::invalid_argument("player ids missing"))?;
        let game_id = self
            .games
            .create_game(game_type, player1, &request.player_ids)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateGameReply { game_id }))
    }

    async fn make_turn(&self, request: Request<MakeTurnRequest>) -> RpcResult<MakeTurnReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type = GameType::try_from(request.game_type)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        // For now, it's a creator id
        let game_id = request.game_id;
        let game_state = self
            .games
            .update_game(game_type, game_id, request.player_id, &request.turn_data)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(MakeTurnReply {
            game_state: Some(game_state),
        }))
    }

    type MakeTurnStreamingStream =
        Pin<Box<dyn Stream<Item = Result<MakeTurnReply, Status>> + Send + 'static>>;

    async fn make_turn_streaming(
        &self,
        request: Request<Streaming<MakeTurnRequest>>,
    ) -> RpcResult<Self::MakeTurnStreamingStream> {
        println!("Got streaming MakeTurn request");

        let mut input_stream = request.into_inner();
        let games = Arc::clone(&self.games);
        let out_stream = async_stream::try_stream! {
            while let Some(req) = input_stream.next().await {
                let request = req?;
                println!("Got request game={}, player={}", request.game_id, request.player_id);

                let game_type = GameType::try_from(request.game_type)
                    .map_err(|e| Status::invalid_argument(e.to_string()))?;
                // For now, it's a creator id
                let game_id = request.game_id;
                let game_state = games.update_game(game_type, game_id, request.player_id, &request.turn_data)
                    .map_err(|e| Status::internal(e.to_string()))?;

                yield MakeTurnReply { game_state: Some(game_state) };
            }
        };

        Ok(Response::new(Box::pin(out_stream)))
    }

    async fn delete_game(&self, request: Request<DeleteGameRequest>) -> RpcResult<DeleteGameReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type = GameType::try_from(request.game_type)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        // For now, it's a creator id
        let game_id = request.game_id;
        self.games
            .delete_game(game_type, game_id)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DeleteGameReply {}))
    }

    async fn get_player_games(
        &self,
        request: Request<GetPlayerGamesRequest>,
    ) -> RpcResult<GetPlayerGamesReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type = GameType::try_from(request.game_type)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let player_games = self
            .games
            .get_player_games(game_type, request.player_id)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(GetPlayerGamesReply {
            games: player_games,
        }))
    }
}
