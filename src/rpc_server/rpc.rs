pub mod game_proto {
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/proto/game.rs"));
}

use tonic::{Request, Response, Status};

use game_proto::game_server::Game;
use game_proto::{CreateGameReply, CreateGameRequest, MakeTurnReply, MakeTurnRequest};

pub type GameId = u64;

#[derive(Debug, Default)]
pub struct GameService {
}

#[tonic::async_trait]
impl Game for GameService {
    async fn create_game(
        &self,
        request: Request<CreateGameRequest>,
    ) -> Result<Response<CreateGameReply>, Status> {
        println!("Got request {:?}", request);

        let reply = CreateGameReply { game_id: 0 };
        Ok(Response::new(reply))
    }

    async fn make_turn(
        &self,
        request: Request<MakeTurnRequest>,
    ) -> Result<Response<MakeTurnReply>, Status> {
        println!("Got request {:?}", request);

        let reply = MakeTurnReply { next_player_id: 0 };
        Ok(Response::new(reply))
    }
}
