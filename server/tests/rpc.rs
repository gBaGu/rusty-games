extern crate server;

use std::net::SocketAddr;
use std::str::FromStr;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::task::JoinHandle;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tonic::metadata::MetadataValue;
use tonic::transport::{server::TcpIncoming, Channel, Server};
use tonic::{Code, Request};

use server::core::{BoardCell, GridIndex, ToProtobuf};
use server::proto::game_client::GameClient;
use server::proto::game_server::GameServer;
use server::proto::*;
use server::rpc_server::{GameImpl, UserId};

fn mock_auth<T>(request: &mut Request<T>, user: UserId) {
    request.metadata_mut().insert(
        "user-id",
        MetadataValue::from_str(&user.to_string()).unwrap(),
    );
}

fn create_game_session_request_stream<S, T>(
    game_type: i32,
    game: u64,
    player: u64,
    move_sequence: S,
    mut ready_receiver: UnboundedReceiver<()>,
) -> impl Stream<Item = GameSessionRequest>
where
    S: IntoIterator<Item = T>,
    T: ToProtobuf,
{
    async_stream::stream! {
        yield GameSessionRequest::init(game_type, game, player);

        let mut moves = move_sequence.into_iter();
        // wait here even if there is no next move so the stream is not finished
        while ready_receiver.recv().await.is_some() {
            let Some(m) = moves.next() else {
                break;
            };
            yield GameSessionRequest::turn_data(m.to_protobuf().unwrap())
        }
    }
}

async fn run_server(addr: &str) -> (JoinHandle<()>, CancellationToken) {
    let ct = CancellationToken::new();
    let ct_cloned = ct.clone();
    let addr: SocketAddr = addr.parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let incoming = TcpIncoming::from_listener(listener, true, None).unwrap();
    let t = tokio::spawn(async move {
        let mut game_impl = GameImpl::default();
        let workers = game_impl.start_workers(ct_cloned);
        Server::builder()
            .add_service(GameServer::new(game_impl))
            .serve_with_incoming_shutdown(incoming, async move {
                if let Err(err) = workers.await {
                    println!("workers join task failed: {}", err);
                };
            })
            .await
            .unwrap();
    });
    return (t, ct);
}

/// Creates a game with id=1 and player_ids=[1, 2]
async fn create_tic_tac_toe_game(client: &mut GameClient<Channel>, players: &[u64]) {
    let mut request = Request::new(CreateGameRequest::new(1, players.to_vec()));
    mock_auth(&mut request, players[0]);
    client.create_game(request).await.unwrap();
}

#[serial_test::serial]
#[tokio::test]
async fn token_cancellation_shuts_down_server() {
    let addr = "127.0.0.1:50051";
    let (server_thread, ct) = run_server(addr).await;
    let mut client = GameClient::connect(format!("http://{}", addr))
        .await
        .unwrap();

    let test_request = GetPlayerGamesRequest::new(1, 1);
    let req = Request::new(test_request);
    let games = client.get_player_games(req).await.unwrap().into_inner();
    assert!(games.games.is_empty());

    ct.cancel();
    server_thread.await.unwrap();

    let req = Request::new(test_request);
    let err = client.get_player_games(req).await.unwrap_err();
    assert_eq!(err.code(), Code::Unavailable);
}

#[serial_test::serial]
#[tokio::test]
async fn game_session_invalid_request() {
    let addr = "127.0.0.1:50051";
    let (server_thread, ct) = run_server(addr).await;
    let mut client = GameClient::connect(format!("http://{}", addr))
        .await
        .unwrap();
    create_tic_tac_toe_game(&mut client, &[1, 2]).await;

    // request oneof value is not set
    let request = Request::new(tokio_stream::once(GameSessionRequest { request: None }));
    let status = client.game_session(request).await.unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);

    // invalid game type
    let mut request = Request::new(tokio_stream::once(GameSessionRequest::init(0, 1, 1)));
    mock_auth(&mut request, 1);
    let status = client.game_session(request).await.unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);

    ct.cancel();
    server_thread.await.unwrap();
}

#[serial_test::serial]
#[tokio::test]
async fn game_session_unexpected_stream_request() {
    let addr = "127.0.0.1:50051";
    let (server_thread, ct) = run_server(addr).await;
    let mut client = GameClient::connect(format!("http://{}", addr))
        .await
        .unwrap();
    create_tic_tac_toe_game(&mut client, &[1, 2]).await;

    // first request is not Init
    let request = tokio_stream::once(GameSessionRequest::turn_data(vec![]));
    let status = client.game_session(request).await.unwrap_err();
    assert_eq!(status.code(), Code::FailedPrecondition);

    // two Init requests in a row
    let mut request = Request::new(tokio_stream::iter([
        GameSessionRequest::init(1, 1, 1),
        GameSessionRequest::init(1, 1, 1),
    ]));
    mock_auth(&mut request, 1);
    let mut stream = client.game_session(request).await.unwrap().into_inner();
    assert_eq!(
        stream.next().await.unwrap().unwrap_err().code(),
        Code::FailedPrecondition
    );
    assert!(stream.next().await.is_none()); // stream is finished after the error

    // two turns in a row
    let mut request = Request::new(tokio_stream::iter([
        GameSessionRequest::init(1, 1, 1),
        GameSessionRequest::turn_data(GridIndex::new(0, 0).to_protobuf().unwrap()),
        GameSessionRequest::turn_data(GridIndex::new(0, 1).to_protobuf().unwrap()),
    ]));
    mock_auth(&mut request, 1);
    let mut stream = client.game_session(request).await.unwrap().into_inner();
    assert!(stream.next().await.unwrap().is_ok());
    assert_eq!(
        stream.next().await.unwrap().unwrap_err().code(),
        Code::Internal
    );
    assert!(stream.next().await.is_none()); // stream is finished after the error

    ct.cancel();
    server_thread.await.unwrap();
}

#[serial_test::serial]
#[tokio::test]
async fn game_session_ttt_success() {
    let addr = "127.0.0.1:50051";
    let (server_thread, ct) = run_server(addr).await;
    let mut client = GameClient::connect(format!("http://{}", addr))
        .await
        .unwrap();
    create_tic_tac_toe_game(&mut client, &[1, 2]).await;

    let player1_moves: Vec<GridIndex> =
        vec![(1, 1).into(), (0, 2).into(), (0, 0).into(), (2, 2).into()];
    let player2_moves: Vec<GridIndex> = vec![(1, 0).into(), (2, 0).into(), (0, 1).into()];

    let (p1_ready_sender, p1_ready_receiver) = unbounded_channel();
    let (p2_ready_sender, p2_ready_receiver) = unbounded_channel();
    let player1_requests =
        create_game_session_request_stream(1, 1, 1, player1_moves, p1_ready_receiver);
    let player2_requests =
        create_game_session_request_stream(1, 1, 2, player2_moves, p2_ready_receiver);

    let mut client_cloned = client.clone();
    let player1 = tokio::spawn(async move {
        let mut request = Request::new(player1_requests);
        mock_auth(&mut request, 1);
        let reply_stream = client_cloned.game_session(request).await.unwrap();
        let mut stream = reply_stream.into_inner();
        println!("> player1 ready to make move");
        p1_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        println!("> player1 ready to make move");
        p1_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        println!("> player1 ready to make move");
        p1_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        println!("> player1 ready to make move");
        p1_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        assert!(stream.next().await.is_none()); // check that server has finished sending replies
    });
    let mut client_cloned = client.clone();
    let player2 = tokio::spawn(async move {
        let mut request = Request::new(player2_requests);
        mock_auth(&mut request, 2);
        let reply_stream = client_cloned.game_session(request).await.unwrap();
        let mut stream = reply_stream.into_inner();
        // for the first turn second player needs to wait for 1 notification response
        stream.next().await.unwrap().unwrap();
        println!("> player2 ready to make move");
        p2_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        println!("> player2 ready to make move");
        p2_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        println!("> player2 ready to make move");
        p2_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        assert!(stream.next().await.is_none()); // check that server has finished sending replies
    });
    player1.await.unwrap();
    player2.await.unwrap();

    let request = Request::new(GetGameRequest::new(1, 1));
    let res = client.get_game(request).await.unwrap();
    let game_info = res.into_inner().game_info.unwrap();
    assert_eq!(game_info.game_id, 1);
    assert_eq!(
        game_info.game_state,
        Some(GameState {
            next_player_id: None,
            winner: Some(0),
        })
    );
    itertools::assert_equal(game_info.players, vec![1, 2]);
    itertools::assert_equal(
        game_info.board,
        vec![
            BoardCell(Some(0u32)).to_protobuf().unwrap(),
            BoardCell(Some(1u32)).to_protobuf().unwrap(),
            BoardCell(Some(0u32)).to_protobuf().unwrap(),
            BoardCell(Some(1u32)).to_protobuf().unwrap(),
            BoardCell(Some(0u32)).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell(Some(1u32)).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell(Some(0u32)).to_protobuf().unwrap(),
        ],
    );

    ct.cancel();
    server_thread.await.unwrap();
}

#[serial_test::serial]
#[tokio::test]
async fn game_session_ttt_session_retry_success() {
    let addr = "127.0.0.1:50051";
    let (server_thread, ct) = run_server(addr).await;
    let mut client = GameClient::connect(format!("http://{}", addr))
        .await
        .unwrap();
    create_tic_tac_toe_game(&mut client, &[1, 2]).await;

    let player1_moves_session1: Vec<GridIndex> = vec![(1, 1).into(), (0, 2).into()];
    let player1_moves_session2: Vec<GridIndex> = vec![(0, 0).into(), (2, 2).into()];
    let player2_moves: Vec<GridIndex> = vec![(1, 0).into(), (2, 0).into(), (0, 1).into()];

    let (p1_ready_sender_session1, p1_ready_receiver_session1) = unbounded_channel();
    let (p1_ready_sender_session2, p1_ready_receiver_session2) = unbounded_channel();
    let (p2_ready_sender, p2_ready_receiver) = unbounded_channel();
    let player1_requests_session1 = create_game_session_request_stream(
        1,
        1,
        1,
        player1_moves_session1,
        p1_ready_receiver_session1,
    );
    let player1_requests_session2 = create_game_session_request_stream(
        1,
        1,
        1,
        player1_moves_session2,
        p1_ready_receiver_session2,
    );
    let player2_requests =
        create_game_session_request_stream(1, 1, 2, player2_moves, p2_ready_receiver);

    let mut client_cloned = client.clone();
    let player1 = tokio::spawn(async move {
        let mut request = Request::new(player1_requests_session1);
        mock_auth(&mut request, 1);
        let reply_stream = client_cloned.game_session(request).await.unwrap();
        let mut stream = reply_stream.into_inner();
        println!("> player1 ready to make move");
        p1_ready_sender_session1.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        println!("> player1 ready to make move");
        p1_ready_sender_session1.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        p1_ready_sender_session1.send(()).unwrap(); // send one more so the stream is finished
        assert!(stream.next().await.is_none()); // reply stream is finished as well

        //make another session
        let mut request = Request::new(player1_requests_session2);
        mock_auth(&mut request, 1);
        let reply_stream = client_cloned.game_session(request).await.unwrap();
        let mut stream = reply_stream.into_inner();
        println!("> player1 ready to make move");
        p1_ready_sender_session2.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        println!("> player1 ready to make move");
        p1_ready_sender_session2.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        assert!(stream.next().await.is_none()); // check that server has finished sending replies
    });
    let mut client_cloned = client.clone();
    let player2 = tokio::spawn(async move {
        let mut request = Request::new(player2_requests);
        mock_auth(&mut request, 2);
        let reply_stream = client_cloned.game_session(request).await.unwrap();
        let mut stream = reply_stream.into_inner();
        // for the first turn second player needs to wait for 1 notification response
        stream.next().await.unwrap().unwrap();
        println!("> player2 ready to make move");
        p2_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        println!("> player2 ready to make move");
        p2_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        println!("> player2 ready to make move");
        p2_ready_sender.send(()).unwrap();
        stream.next().await.unwrap().unwrap();
        stream.next().await.unwrap().unwrap();
        assert!(stream.next().await.is_none()); // check that server has finished sending replies
    });
    player1.await.unwrap();
    player2.await.unwrap();

    let request = Request::new(GetGameRequest::new(1, 1));
    let res = client.get_game(request).await.unwrap();
    let game_info = res.into_inner().game_info.unwrap();
    assert_eq!(game_info.game_id, 1);
    assert_eq!(
        game_info.game_state,
        Some(GameState {
            next_player_id: None,
            winner: Some(0),
        })
    );
    itertools::assert_equal(game_info.players, vec![1, 2]);
    itertools::assert_equal(
        game_info.board,
        vec![
            BoardCell(Some(0u32)).to_protobuf().unwrap(),
            BoardCell(Some(1u32)).to_protobuf().unwrap(),
            BoardCell(Some(0u32)).to_protobuf().unwrap(),
            BoardCell(Some(1u32)).to_protobuf().unwrap(),
            BoardCell(Some(0u32)).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell(Some(1u32)).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell(Some(0u32)).to_protobuf().unwrap(),
        ],
    );

    ct.cancel();
    server_thread.await.unwrap();
}

#[serial_test::serial]
#[tokio::test]
async fn game_session_turn_notification_on_single_request() {
    let addr = "127.0.0.1:50051";
    let (server_thread, ct) = run_server(addr).await;
    let mut client = GameClient::connect(format!("http://{}", addr))
        .await
        .unwrap();
    create_tic_tac_toe_game(&mut client, &[1, 2]).await;

    let (p1_ready_sender, p1_ready_receiver) = unbounded_channel();
    let only_init =
        create_game_session_request_stream(1, 1, 1, Vec::<GridIndex>::new(), p1_ready_receiver);
    let mut request = Request::new(only_init);
    mock_auth(&mut request, 1);
    let reply_stream = client.game_session(request).await.unwrap();
    let mut stream = reply_stream.into_inner();

    // send single turn request
    let mut request = Request::new(MakeTurnRequest::new(
        1,
        1,
        1,
        GridIndex::new(1, 1).to_protobuf().unwrap(),
    ));
    mock_auth(&mut request, 1);
    client.make_turn(request).await.unwrap();
    // check that notification is received
    stream.next().await.unwrap().unwrap();
    p1_ready_sender.send(()).unwrap(); // end request stream
    assert!(stream.next().await.is_none());

    let request = Request::new(GetGameRequest::new(1, 1));
    let res = client.get_game(request).await.unwrap();
    let game_info = res.into_inner().game_info.unwrap();
    assert_eq!(game_info.game_id, 1);
    assert_eq!(
        game_info.game_state,
        Some(GameState {
            next_player_id: Some(1),
            winner: None,
        })
    );
    itertools::assert_equal(game_info.players, vec![1, 2]);
    itertools::assert_equal(
        game_info.board,
        vec![
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell(Some(0u32)).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
            BoardCell::<u32>(None).to_protobuf().unwrap(),
        ],
    );

    ct.cancel();
    server_thread.await.unwrap();
}

#[serial_test::serial]
#[tokio::test]
async fn get_player_games() {
    let addr = "127.0.0.1:50051";
    let (server_thread, ct) = run_server(addr).await;
    let mut client = GameClient::connect(format!("http://{}", addr))
        .await
        .unwrap();
    create_tic_tac_toe_game(&mut client, &[1, 2]).await;
    create_tic_tac_toe_game(&mut client, &[6, 1]).await;
    create_tic_tac_toe_game(&mut client, &[2, 1]).await;
    create_tic_tac_toe_game(&mut client, &[4, 5]).await;

    let request = Request::new(GetPlayerGamesRequest::new(1, 1));
    let games = client.get_player_games(request).await.unwrap().into_inner();
    assert_eq!(games.games.len(), 3);
    let game = games.games.iter().find(|g| g.game_id == 1).unwrap();
    assert_eq!(game.players, vec![1, 2]);
    let game = games.games.iter().find(|g| g.game_id == 2).unwrap();
    assert_eq!(game.players, vec![2, 1]);
    let game = games.games.iter().find(|g| g.game_id == 6).unwrap();
    assert_eq!(game.players, vec![6, 1]);

    let request = Request::new(GetPlayerGamesRequest::new(1, 2));
    let games = client.get_player_games(request).await.unwrap().into_inner();
    assert_eq!(games.games.len(), 2);
    let game = games.games.iter().find(|g| g.game_id == 1).unwrap();
    assert_eq!(game.players, vec![1, 2]);
    let game = games.games.iter().find(|g| g.game_id == 2).unwrap();
    assert_eq!(game.players, vec![2, 1]);

    let request = Request::new(GetPlayerGamesRequest::new(1, 5));
    let games = client.get_player_games(request).await.unwrap().into_inner();
    assert_eq!(games.games.len(), 1);
    assert_eq!(games.games[0].players, vec![4, 5]);

    ct.cancel();
    server_thread.await.unwrap();
}
