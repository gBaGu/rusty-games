pub mod game_proto {
    tonic::include_proto!("game");
    pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("game_descriptor");
}

use std::collections::hash_map::{Entry, HashMap};
use std::sync::Mutex;

use tonic::{Request, Response, Status};

use crate::game::tic_tac_toe::{FieldCol, FieldCoordinates, FieldRow, PlayerId, TicTacToe};
use game_proto::game_server::Game;
use game_proto::{CreateGameReply, CreateGameRequest, MakeTurnReply, MakeTurnRequest};

pub type RpcResult<T> = Result<Response<T>, Status>;

#[derive(Debug, Default)]
pub struct GameService {
    games: Mutex<HashMap<PlayerId, TicTacToe>>,
}

#[tonic::async_trait]
impl Game for GameService {
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
        let mut game_lock = self
            .games
            .lock()
            .map_err(|e| Status::internal(e.to_string()))?;
        let game = game_lock
            .get_mut(&game_id)
            .ok_or_else(|| Status::invalid_argument("game not found"))?;
        let row = FieldRow::try_from(request.get_ref().row as usize)
            .map_err(|e| Status::internal(e.to_string()))?;
        let col = FieldCol::try_from(request.get_ref().col as usize)
            .map_err(|e| Status::internal(e.to_string()))?;
        let coords = FieldCoordinates::new(row, col);
        game.make_turn(request.get_ref().player_id, coords)
            .map_err(|e| Status::internal(e.to_string()))?;

        let reply = MakeTurnReply {
            next_player_id: game.get_current_player().get_id(),
        };
        Ok(Response::new(reply))
    }
}
