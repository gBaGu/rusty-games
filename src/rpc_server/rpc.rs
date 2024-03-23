pub mod game_proto {
    use crate::game::tic_tac_toe::{FinishedState, GameState};

    tonic::include_proto!("game");
    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("game_descriptor");

    impl MakeTurnReply {
        pub fn from_game_state(state: GameState) -> Self {
            match state {
                GameState::Turn(id) => Self {
                    next_player_id: Some(id),
                    ..Default::default()
                },
                GameState::Finished(FinishedState::Win(id)) => Self {
                    winner: Some(id),
                    ..Default::default()
                },
                GameState::Finished(FinishedState::Draw) => Self::default(),
            }
        }
    }
}

use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::game::tic_tac_toe::{FieldCol, FieldCoordinates, FieldRow};
use crate::rpc_server::game_storage::GameStorage;
use game_proto::game_server::Game;
use game_proto::{
    CreateGameReply, CreateGameRequest, DeleteGameReply, DeleteGameRequest, MakeTurnReply,
    MakeTurnRequest,
};

pub type RpcResult<T> = Result<Response<T>, Status>;

#[derive(Debug, Default)]
pub struct GameImpl {
    games: Arc<GameStorage>,
}

#[tonic::async_trait]
impl Game for GameImpl {
    async fn create_game(&self, request: Request<CreateGameRequest>) -> RpcResult<CreateGameReply> {
        println!("Got request {:?}", request);

        if request.get_ref().player_ids.len() != 2 {
            return Err(Status::invalid_argument(
                "invalid number of players (expected 2)",
            ));
        }
        let player1 = request.get_ref().player_ids[0];
        let player2 = request.get_ref().player_ids[1];
        self.games
            .create_game(player1, player2)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(CreateGameReply { game_id: player1 }))
    }

    async fn make_turn(&self, request: Request<MakeTurnRequest>) -> RpcResult<MakeTurnReply> {
        println!("Got request {:?}", request);

        // For now, it's a creator id
        let game_id = request.get_ref().game_id;
        let row = FieldRow::try_from(request.get_ref().row as usize)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let col = FieldCol::try_from(request.get_ref().col as usize)
            .map_err(|e| Status::invalid_argument(e.to_string()))?;
        let coords = FieldCoordinates::new(row, col);
        let game_state = self
            .games
            .update_game(game_id, request.get_ref().player_id, coords)
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
                let req = req?;
                println!(
                    "Got request game={}, player={}, row={}, col={}",
                    req.game_id, req.player_id, req.row, req.col
                );

                // For now, it's a creator id
                let game_id = req.game_id;
                let row = FieldRow::try_from(req.row as usize)
                    .map_err(|e| Status::invalid_argument(e.to_string()))?;
                let col = FieldCol::try_from(req.col as usize)
                    .map_err(|e| Status::invalid_argument(e.to_string()))?;
                let coords = FieldCoordinates::new(row, col);
                let game_state = games.update_game(game_id, req.player_id, coords)
                    .map_err(|e| Status::internal(e.to_string()))?;

                yield MakeTurnReply::from_game_state(game_state);
            }
        };

        Ok(Response::new(Box::pin(out_stream)))
    }

    async fn delete_game(&self, request: Request<DeleteGameRequest>) -> RpcResult<DeleteGameReply> {
        println!("Got request {:?}", request);

        // For now, it's a creator id
        let game_id = request.get_ref().game_id;
        self.games
            .delete_game(game_id)
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(DeleteGameReply {}))
    }
}
