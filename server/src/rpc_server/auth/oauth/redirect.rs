use std::future::{Future, IntoFuture};
use std::net::SocketAddr;
use std::sync::Arc;

use oauth2::basic::BasicTokenResponse;
use oauth2::{reqwest, url};
use oauth2::{AuthorizationCode, PkceCodeVerifier, CsrfToken};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

use super::{OAuth2Manager, OAuth2Meta};

const REDIRECT_REPLY_SUCCESS: &str = "Success!\nYou may now close this page.";
const REDIRECT_REPLY_ERROR: &str = "Something went wrong!";

pub struct RedirectListener(JoinHandle<()>);

impl IntoFuture for RedirectListener {
    type Output = <JoinHandle<()> as Future>::Output;
    type IntoFuture = JoinHandle<()>;

    fn into_future(self) -> Self::IntoFuture {
        self.0.into_future()
    }
}

impl RedirectListener {
    pub fn new(
        addr: SocketAddr,
        auth_manager: Arc<OAuth2Manager>,
        token_sender: mpsc::UnboundedSender<(OAuth2Meta, BasicTokenResponse)>,
        ct: CancellationToken,
    ) -> Self {
        let inner = tokio::spawn(async move {
            let listener = match TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(err) => {
                    println!("redirect listener: failed to bind to {}: {}", addr, err);
                    return;
                }
            };
            let mut listener = TcpListenerStream::new(listener);
            loop {
                tokio::select! {
                    biased;
                    _ = ct.cancelled() => {
                        println!("redirect listener: cancelled");
                        break;
                    }
                    next = listener.next() => {
                        let Some(stream) = next else {
                            break;
                        };
                        match stream {
                            Ok(stream) => {
                                println!("redirect listener: received connection");
                                tokio::spawn(handle_connection(stream, auth_manager.clone(), token_sender.clone()));
                            }
                            Err(err) => {
                                println!("redirect listener: connection failed: {}", err);
                            }
                        }
                    }
                }
            }
            println!("redirect listener: finished");
        });
        Self(inner)
    }
}

async fn send_tcp_stream_reply(stream: &mut TcpStream, message: &str) {
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
        message.len(),
        message
    );
    if let Err(err) = stream.write_all(response.as_bytes()).await {
        println!("failed to send reply message: {}", err);
    }
}

async fn handle_connection(
    stream: TcpStream,
    auth_manager: Arc<OAuth2Manager>,
    token_sender: mpsc::UnboundedSender<(OAuth2Meta, BasicTokenResponse)>,
) {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();
    if let Err(err) = reader.read_line(&mut request_line).await {
        println!("handle_connection: unable to read line: {}", err);
        send_tcp_stream_reply(&mut reader.into_inner(), REDIRECT_REPLY_ERROR).await;
        return;
    }
    let mut stream = reader.into_inner();
    let Some(redirect_url) = request_line.split_whitespace().nth(1) else {
        println!("handle_connection: unexpected format");
        send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_ERROR).await;
        return;
    };
    let redirect_url = match url::Url::parse(&format!("http://localhost{}", redirect_url)) {
        Ok(url) => url,
        Err(err) => {
            println!("handle_connection: unable to parse redirect url: {}", err);
            send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_ERROR).await;
            return;
        }
    };

    let code = match redirect_url
        .query_pairs()
        .find(|(key, _)| key == "code")
        .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
    {
        Some(code) => code,
        None => {
            println!("handle_connection: unable to parse authorization code");
            send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_ERROR).await;
            return;
        }
    };
    let state = match redirect_url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .map(|(_, state)| CsrfToken::new(state.into_owned()))
    {
        Some(token) => token,
        None => {
            println!("handle_connection: unable to parse state");
            send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_ERROR).await;
            return;
        }
    };
    let meta = match auth_manager.remove(&state) {
        Ok(Some(meta)) => meta,
        Ok(None) => {
            println!("handle_connection: auth metadata is missing");
            send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_ERROR).await;
            return;
        }
        Err(err) => {
            println!("handle_connection: unable to get auth metadata: {}", err);
            send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_ERROR).await;
            return;
        }
    };
    let pkce_verifier = PkceCodeVerifier::new(meta.pkce_verifier().secret().clone());

    let http_client = match reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            println!("handle_connection: unable to create http client: {}", err);
            send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_ERROR).await;
            return;
        }
    };
    let token_response = match auth_manager
        .client()
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(&http_client)
        .await
    {
        Ok(response) => response,
        Err(err) => {
            println!("handle_connection: unable to get token: {}", err);
            send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_ERROR).await;
            return;
        }
    };
    if let Err(err) = token_sender.send((meta, token_response)) {
        println!("handle_connection: unable to send oauth2 token: {}", err);
        send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_ERROR).await;
        return;
    }
    send_tcp_stream_reply(&mut stream, REDIRECT_REPLY_SUCCESS).await;
}
