use std::collections::HashMap;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio::task::AbortHandle;
use tokio::time::sleep;
use tungstenite::connect;
use tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use url::Url;

use crate::app::Action;
use crate::parser::parse_raw_trace;

use tungstenite::Message;

type Tx = UnboundedSender<Message>;

pub struct ConnectionMeta {
    tx: Tx,
    path: String,
}
pub type PeerMap = Arc<std::sync::Mutex<HashMap<SocketAddr, ConnectionMeta>>>;

#[derive(Default)]
struct RequestPath {
    uri: String,
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

#[derive(Default, Debug)]
pub struct WebSocketState {
    main_abort_handle: Option<AbortHandle>,
    handles: Vec<AbortHandle>,
}

#[derive(Default)]
pub struct WebSocket {
    address: String,
    peer_map: PeerMap,
    web_socket_state: Arc<Mutex<WebSocketState>>,
    open: bool,
}

impl WebSocket {
    pub fn new() -> WebSocket {
        let address = "127.0.0.1:9999".to_string();

        let peer_map = PeerMap::new(std::sync::Mutex::new(HashMap::new()));

        WebSocket {
            open: false,
            address,
            peer_map,
            web_socket_state: Arc::new(Mutex::new(WebSocketState {
                handles: vec![],
                main_abort_handle: None,
            })),
        }
    }

    pub async fn start(&mut self, tx: tokio::sync::mpsc::UnboundedSender<Action>) {
        if !self.open {
            let address = &self.address;

            self.peer_map = PeerMap::new(std::sync::Mutex::new(HashMap::new()));

            let try_socket = TcpListener::bind(address).await;

            let listener = try_socket.expect("Failed to bind");

            let peer_map = &self.peer_map;

            let cloned_peer_map = peer_map.clone();

            let websocket_state = &self.web_socket_state;

            let cloned_websocket_state = websocket_state.clone();

            let main_join_handle = tokio::spawn(async move {
                while let Ok((stream, addr)) = listener.accept().await {
                    let join_handle = tokio::spawn(handle_connection(
                        cloned_peer_map.clone(),
                        stream,
                        addr,
                        tx.clone(),
                    ));

                    cloned_websocket_state
                        .clone()
                        .lock()
                        .await
                        .handles
                        .push(join_handle.abort_handle());
                }
            });

            let state = &mut self.web_socket_state.lock().await;

            state.main_abort_handle = Some(main_join_handle.abort_handle());

            self.open = true;
        }
    }

    pub async fn stop(&mut self) -> Result<(), String> {
        if self.open {
            self.open = false;

            let websocket_state = &mut self.web_socket_state.lock().await;

            websocket_state.main_abort_handle.as_ref().unwrap().abort();

            websocket_state.handles.iter().for_each(|x| {
                x.abort();
            });

            websocket_state.handles.clear();

            websocket_state.main_abort_handle = None;
        }

        Ok(())
    }
}

pub async fn client(
    tx: Option<tokio::sync::mpsc::UnboundedSender<Action>>,
) -> Result<(), Box<dyn Error>> {
    let (mut socket, _response) =
        connect(Url::parse("ws://127.0.0.1:9999/inner_client").unwrap()).expect("Can't connect");

    loop {
        let msg = socket.read();

        match msg {
            Ok(message) => {
                match message {
                    tungstenite::Message::Text(s) => {
                        match parse_raw_trace(&s) {
                            Ok(request) => match request {
                                crate::parser::Payload::Trace(trace) => {
                                    let mut should_persist = true;

                                    let http_trace = trace.http.as_ref().unwrap();

                                    if let Some(port) = &http_trace.port {
                                        if port == "9999" {
                                            should_persist = false;
                                        }
                                    }

                                    if let Some(s) = tx.clone() {
                                        let id = trace.id.clone();
                                        let s1 = s.clone();
                                        tokio::spawn(async move {
                                            sleep(Duration::from_millis(5000)).await;
                                            s1.send(Action::MarkTraceAsTimedOut(id)).unwrap();
                                        });

                                        if should_persist {
                                            let s2 = s.clone();
                                            s2.send(Action::AddTrace(trace)).unwrap();
                                        }
                                    }
                                }
                                _ => {}
                            },
                            Err(err) => {
                                println!("Trace NOT parsed!! {:?}", err)
                            }
                        };
                    }
                    tungstenite::Message::Close(_) => {}
                    _ => {}
                };
            }
            Err(_e) => {}
        }
    }
}

pub async fn handle_connection(
    peer_map: PeerMap,
    raw_stream: TcpStream,
    addr: SocketAddr,
    action_sender: tokio::sync::mpsc::UnboundedSender<Action>,
) {
    let mut path_rewrite_callback = RequestPath::default();

    let ws_stream = tokio_tungstenite::accept_hdr_async(raw_stream, &mut path_rewrite_callback)
        .await
        .expect("Error during the websocket handshake occurred");

    let path = path_rewrite_callback.uri;

    let cloned_path = path.clone();

    let (tx, rx) = unbounded();

    peer_map.lock().unwrap().insert(
        addr,
        ConnectionMeta {
            tx,
            path: cloned_path.clone(),
        },
    );

    let _ = action_sender.send(Action::AddClient(crate::app::WssClient {
        path: path.clone(),
        address: addr.to_string(),
    }));

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        let peers = peer_map.lock().unwrap();

        // We want to broadcast the message to everyone except ourselves.
        let broadcast_recipients = peers
            .iter()
            .filter(|(peer_addr, _)| peer_addr != &&addr)
            .map(|(_, ws_sink)| ws_sink);

        for recp in broadcast_recipients {
            recp.tx.unbounded_send(msg.clone()).unwrap();
        }

        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    let _ = action_sender.send(Action::SetGeneralStatus(format!(
        "Client {} got disconnected",
        path
    )));

    if path == "/inner_client" {
        let s = action_sender.clone();
        tokio::spawn(async {
            let _ = client(Some(s)).await;
        });
    }

    peer_map.lock().unwrap().remove(&addr);

    let _ = action_sender.send(Action::RemoveClient(crate::app::WssClient {
        path: path.clone(),
        address: addr.to_string(),
    }));
}
