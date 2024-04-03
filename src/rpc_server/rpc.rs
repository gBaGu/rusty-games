use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use super::game_proto::{
    game_server::Game, CreateGameReply, CreateGameRequest, DeleteGameReply, DeleteGameRequest,
    GameType, MakeTurnReply, MakeTurnRequest,
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
        if request.player_ids.len() != 2 {
            return Err(Status::invalid_argument(
                "invalid number of players (expected 2)",
            ));
        }
        let game_type = GameType::try_from(request.game_type)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let player1 = request.player_ids[0];
        let player2 = request.player_ids[1];
        self.games
            .create_game(game_type, player1, player2)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateGameReply { game_id: player1 }))
    }

    async fn make_turn(&self, request: Request<MakeTurnRequest>) -> RpcResult<MakeTurnReply> {
        println!("Got request {:?}", request);

        let request = request.into_inner();
        let game_type = GameType::try_from(request.game_type)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        // For now, it's a creator id
        let game_id = request.game_id;
        let coords = (request.row, request.col);
        let game_state = self
            .games
            .update_game(game_type, game_id, request.player_id, coords)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(MakeTurnReply::from_game_state(game_state)))
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
                println!(
                    "Got request game={}, player={}, row={}, col={}",
                    request.game_id, request.player_id, request.row, request.col
                );

                let game_type = GameType::try_from(request.game_type)
                    .map_err(|e| Status::invalid_argument(e.to_string()))?;
                // For now, it's a creator id
                let game_id = request.game_id;
                let coords = (request.row, request.col);
                let game_state = games.update_game(game_type, game_id, request.player_id, coords)
                    .map_err(|e| Status::internal(e.to_string()))?;

                yield MakeTurnReply::from_game_state(game_state);
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
}
