pub mod game_proto {
    tonic::include_proto!("game");
    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("game_descriptor");
}

use std::collections::hash_map::{Entry, HashMap};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::game::tic_tac_toe::{
    FieldCol, FieldCoordinates, FieldRow, FinishedState, GameState, PlayerId, TicTacToe,
    TicTacToeError,
};
use game_proto::game_server::Game;
use game_proto::{
    CreateGameReply, CreateGameRequest, DeleteGameReply, DeleteGameRequest, MakeTurnReply,
    MakeTurnRequest,
};

pub type RpcResult<T> = Result<Response<T>, Status>;

// TODO: create separate object that will handle this kind of operations
fn update_game(
    games: &Arc<Mutex<HashMap<PlayerId, TicTacToe>>>,
    game_id: PlayerId,
    player_id: PlayerId,
    coords: FieldCoordinates,
) -> Result<GameState, TicTacToeError> {
    let mut games_guard = games.lock().unwrap();
    let game = games_guard.get_mut(&game_id).unwrap();
    game.make_turn(player_id, coords)
}

#[derive(Debug, Default)]
pub struct GameImpl {
    games: Arc<Mutex<HashMap<PlayerId, TicTacToe>>>,
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
        let mut games_guard = self
            .games
            .lock()
            .map_err(|e| Status::internal(e.to_string()))?;
        match games_guard.entry(player1) {
            Entry::Vacant(e) => {
                let game = TicTacToe::new(player1, player2)
                    .map_err(|e| Status::internal(e.to_string()))?;
                e.insert(game);
            }
            Entry::Occupied(_) => {
                return Err(Status::invalid_argument(
                    "this player already has an active game",
                ));
            }
        }
        drop(games_guard);

        Ok(Response::new(CreateGameReply { game_id: player1 }))
    }

    async fn make_turn(&self, request: Request<MakeTurnRequest>) -> RpcResult<MakeTurnReply> {
        println!("Got request {:?}", request);

        // For now, it's a creator id
        let game_id = request.get_ref().game_id;
        let row = FieldRow::try_from(request.get_ref().row as usize)
            .map_err(|e| Status::internal(e.to_string()))?;
        let col = FieldCol::try_from(request.get_ref().col as usize)
            .map_err(|e| Status::internal(e.to_string()))?;
        let coords = FieldCoordinates::new(row, col);
        let mut games_guard = self
            .games
            .lock()
            .map_err(|e| Status::internal(e.to_string()))?;
        let game = games_guard
            .get_mut(&game_id)
            .ok_or_else(|| Status::invalid_argument("game not found"))?;
        let game_state = game
            .make_turn(request.get_ref().player_id, coords)
            .map_err(|e| Status::internal(e.to_string()))?;

        let mut reply = MakeTurnReply::default();
        match game_state {
            GameState::Turn(id) => reply.next_player_id = Some(id),
            GameState::Finished(FinishedState::Win(id)) => reply.winner = Some(id),
            _ => (),
        }
        Ok(Response::new(reply))
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
                    .map_err(|e| Status::internal(e.to_string()))?;
                let col = FieldCol::try_from(req.col as usize)
                    .map_err(|e| Status::internal(e.to_string()))?;
                let coords = FieldCoordinates::new(row, col);
                let game_state = update_game(&games, game_id, req.player_id, coords)
                    .map_err(|e| Status::internal(e.to_string()))?;

                let mut reply = MakeTurnReply::default();
                match game_state {
                    GameState::Turn(id) => reply.next_player_id = Some(id),
                    GameState::Finished(FinishedState::Win(id)) => reply.winner = Some(id),
                    _ => (),
                }
                yield reply;
            }
        };

        Ok(Response::new(Box::pin(out_stream)))
    }

    async fn delete_game(&self, request: Request<DeleteGameRequest>) -> RpcResult<DeleteGameReply> {
        println!("Got request {:?}", request);

        // For now, it's a creator id
        let game_id = request.get_ref().game_id;
        let mut games_guard = self
            .games
            .lock()
            .map_err(|e| Status::internal(e.to_string()))?;
        if let Entry::Occupied(e) = games_guard.entry(game_id) {
            if !e.get().is_finished() {
                return Err(Status::failed_precondition("game is not finished"));
            }
            e.remove();
        }
        drop(games_guard);

        Ok(Response::new(DeleteGameReply {}))
    }
}
