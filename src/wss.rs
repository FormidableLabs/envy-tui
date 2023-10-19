use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, StreamExt, TryStreamExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tungstenite::connect;
use tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use url::Url;

use crate::app::{App, WsServerState};
use crate::parser::parse_raw_trace;
use crate::TraceTimeoutPayload;

use tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<std::sync::Mutex<HashMap<SocketAddr, Tx>>>;

pub async fn client(app: &Arc<Mutex<App>>, tx: UnboundedSender<TraceTimeoutPayload>) {
    let (mut socket, _response) =
        connect(Url::parse("ws://127.0.0.1:9999/inner_client").unwrap()).expect("Can't connect");

    loop {
        let msg = socket.read().expect("Error reading message");

        let msg = match msg {
            tungstenite::Message::Text(s) => s,
            _ => {
                panic!()
            }
        };

        let mut app_guard = app.lock().await;

        let _ = match parse_raw_trace(&msg) {
            Ok(request) => {
                app_guard.logs.push(request.to_string());

                let id = request.id.clone();

                let cloned_sender = tx.clone();

                tokio::spawn(async move {
                    sleep(Duration::from_millis(5000)).await;

                    cloned_sender.unbounded_send(TraceTimeoutPayload::MarkForTimeout(id))
                });

                app_guard.items.replace(request);
                app_guard.is_first_render = true;

                ()
            }
            Err(err) => {
                println!("Trace NOT parsed!! {:?}", err)
            }
        };
    }
}

struct RequestPath {
    uri: String,
}

impl Default for RequestPath {
    fn default() -> RequestPath {
        RequestPath { uri: String::new() }
    }
}

impl Callback for &mut RequestPath {
    fn on_request(
        mut self,
        request: &Request,
        response: Response,
    ) -> Result<Response, ErrorResponse> {
        self.uri = request.uri().to_string();

        Ok(response)
    }
}

pub async fn handle_connection(
    peer_map: PeerMap,
    raw_stream: TcpStream,
    addr: SocketAddr,
    app: Arc<Mutex<App>>,
) {
    let mut path_rewrite_callback = RequestPath::default();

    let ws_stream = tokio_tungstenite::accept_hdr_async(raw_stream, &mut path_rewrite_callback)
        .await
        .expect("Error during the websocket handshake occurred");
    let path = path_rewrite_callback.uri;

    let (tx, rx) = unbounded();

    peer_map.lock().unwrap().insert(addr, tx);

    if path != "/inner_client" {
        let number_of_connections = peer_map.lock().unwrap().len();

        match number_of_connections - 1 {
            0 => app.lock().await.ws_server_state = WsServerState::Open,
            v => app.lock().await.ws_server_state = WsServerState::HasConnections(v),
        }
    }

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        let peers = peer_map.lock().unwrap();

        // We want to broadcast the message to everyone except ourselves.
        let broadcast_recipients = peers
            .iter()
            .filter(|(peer_addr, _)| peer_addr != &&addr)
            .map(|(_, ws_sink)| ws_sink);

        for recp in broadcast_recipients {
            recp.unbounded_send(msg.clone()).unwrap();
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    // pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    // println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);

    if path != "/inner_client" {
        let number_of_connections = peer_map.lock().unwrap().len();

        match number_of_connections - 1 {
            0 => app.lock().await.ws_server_state = WsServerState::Open,
            v => app.lock().await.ws_server_state = WsServerState::HasConnections(v),
        }
    }
}
