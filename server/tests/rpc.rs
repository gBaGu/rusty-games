extern crate server;

use std::net::SocketAddr;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::task::JoinHandle;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tonic::transport::{Channel, Server};
use tonic::{Code, Request};

use server::game::encoding::ToProtobuf;
use server::game::grid::GridIndex;
use server::game::BoardCell;
use server::proto::game_client::GameClient;
use server::proto::game_server::GameServer;
use server::proto::*;
use server::rpc_server::rpc::GameImpl;

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
        yield GameSessionRequest {
            request: Some(game_session_request::Request::Init(GameSession {
                game_type,
                game_id: game,
                player_id: player,
            }))
        };

        let mut moves = move_sequence.into_iter();
        while ready_receiver.recv().await.is_some() { // wait here even if there is no next move so the stream is not finished
            let Some(m) = moves.next() else {
                break;
            };
            yield GameSessionRequest {
                request: Some(game_session_request::Request::TurnData(m.to_protobuf().unwrap()))
            }
        }
    }
}

async fn run_server(addr: &str) -> (JoinHandle<()>, CancellationToken) {
    let ct = CancellationToken::new();
    let ct_cloned = ct.clone();
    let addr: SocketAddr = addr.parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let incoming =
        tonic::transport::server::TcpIncoming::from_listener(listener, true, None).unwrap();
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
async fn create_test_game(client: &mut GameClient<Channel>) {
    let request = Request::new(CreateGameRequest {
        game_type: 1,
        player_ids: vec![1, 2],
    });
    client.create_game(request).await.unwrap();
}

#[serial_test::serial]
#[tokio::test]
async fn game_session_invalid_request() {
    let addr = "127.0.0.1:50051";
    let (server_thread, ct) = run_server(addr).await;
    let mut client = GameClient::connect(format!("http://{}", addr))
        .await
        .unwrap();
    create_test_game(&mut client).await;

    // request oneof value is not set
    let status = client
        .game_session(tokio_stream::once(GameSessionRequest { request: None }))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);

    // invalid game type
    let status = client
        .game_session(tokio_stream::once(GameSessionRequest {
            request: Some(game_session_request::Request::Init(GameSession {
                game_type: 0,
                game_id: 1,
                player_id: 1,
            })),
        }))
        .await
        .unwrap_err();
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
    create_test_game(&mut client).await;

    // first request is not Init
    let status = client
        .game_session(tokio_stream::once(GameSessionRequest {
            request: Some(game_session_request::Request::TurnData(vec![
                0, 0, 0, 0, 0, 0, 0, 0,
            ])),
        }))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::FailedPrecondition);

    // two Init requests in a row
    let mut stream = client
        .game_session(tokio_stream::iter([
            GameSessionRequest {
                request: Some(game_session_request::Request::Init(GameSession {
                    game_type: 1,
                    game_id: 1,
                    player_id: 1,
                })),
            },
            GameSessionRequest {
                request: Some(game_session_request::Request::Init(GameSession {
                    game_type: 1,
                    game_id: 1,
                    player_id: 1,
                })),
            },
        ]))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        stream.next().await.unwrap().unwrap_err().code(),
        Code::FailedPrecondition
    );

    // two turns in a row
    let mut stream = client
        .game_session(tokio_stream::iter([
            GameSessionRequest {
                request: Some(game_session_request::Request::Init(GameSession {
                    game_type: 1,
                    game_id: 1,
                    player_id: 1,
                })),
            },
            GameSessionRequest {
                request: Some(game_session_request::Request::TurnData(vec![
                    0, 0, 0, 0, 0, 0, 0, 0,
                ])),
            },
            GameSessionRequest {
                request: Some(game_session_request::Request::TurnData(vec![
                    0, 0, 0, 0, 0, 0, 0, 1,
                ])),
            },
        ]))
        .await
        .unwrap()
        .into_inner();
    assert_eq!(
        stream.next().await.unwrap().unwrap_err().code(),
        Code::Internal
    );

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
    create_test_game(&mut client).await;

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
        let reply_stream = client_cloned.game_session(player1_requests).await.unwrap();
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
        let reply_stream = client_cloned.game_session(player2_requests).await.unwrap();
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

    let res = client
        .get_game(Request::new(GetGameRequest {
            game_type: 1,
            game_id: 1,
        }))
        .await
        .unwrap();
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
    create_test_game(&mut client).await;

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
        let reply_stream = client_cloned
            .game_session(player1_requests_session1)
            .await
            .unwrap();
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
        let reply_stream = client_cloned
            .game_session(player1_requests_session2)
            .await
            .unwrap();
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
        let reply_stream = client_cloned.game_session(player2_requests).await.unwrap();
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

    let res = client
        .get_game(Request::new(GetGameRequest {
            game_type: 1,
            game_id: 1,
        }))
        .await
        .unwrap();
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
    create_test_game(&mut client).await;

    let (p1_ready_sender, p1_ready_receiver) = unbounded_channel();
    let only_init =
        create_game_session_request_stream(1, 1, 1, Vec::<GridIndex>::new(), p1_ready_receiver);
    let reply_stream = client.game_session(only_init).await.unwrap();
    let mut stream = reply_stream.into_inner();

    // send single turn request
    client
        .make_turn(Request::new(MakeTurnRequest {
            game_type: 1,
            game_id: 1,
            player_id: 1,
            turn_data: GridIndex::new(1, 1).to_protobuf().unwrap(),
        }))
        .await
        .unwrap();
    // check that notification is received
    stream.next().await.unwrap().unwrap();
    p1_ready_sender.send(()).unwrap(); // end request stream
    assert!(stream.next().await.is_none());

    let res = client
        .get_game(Request::new(GetGameRequest {
            game_type: 1,
            game_id: 1,
        }))
        .await
        .unwrap();
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
