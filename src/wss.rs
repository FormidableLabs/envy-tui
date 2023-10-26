use std::collections::HashMap;
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

use crate::app::{App, AppDispatch};
use crate::parser::parse_raw_trace;

use tungstenite::Message;

type Tx = UnboundedSender<Message>;
pub type PeerMap = Arc<std::sync::Mutex<HashMap<SocketAddr, Tx>>>;

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

struct ConnectionMeta {
    path: String,
}

struct WebSocketState {
    connections: Vec<ConnectionMeta>,
    main_abort_handle: Option<AbortHandle>,
    handles: Vec<AbortHandle>,
}

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
                connections: vec![],
                handles: vec![],
                main_abort_handle: None,
            })),
        }
    }

    pub async fn start(&mut self) {
        if !self.open {
            let address = &self.address;

            self.peer_map = PeerMap::new(std::sync::Mutex::new(HashMap::new()));

            let try_socket = TcpListener::bind(address).await;

            let listener = try_socket.expect("Failed to bind");

            let peer_map = &self.peer_map;

            let cloned_peer_map = peer_map.clone();

            let websocket_state = &mut self.web_socket_state;

            let cloned_websocket_state = websocket_state.clone();

            let main_join_handle = tokio::spawn(async move {
                while let Ok((stream, addr)) = listener.accept().await {
                    let join_handle =
                        tokio::spawn(handle_connection(cloned_peer_map.clone(), stream, addr));

                    cloned_websocket_state
                        .clone()
                        .lock()
                        .await
                        .handles
                        .push(join_handle.abort_handle());
                }

                ()
            });

            let state = &mut self.web_socket_state.lock().await;

            state.main_abort_handle = Some(main_join_handle.abort_handle());

            self.open = true;
        }
    }

    pub fn get_connections(&self) -> usize {
        self.peer_map.lock().unwrap().iter().len()
    }

    pub fn is_open(&self) -> bool {
        self.open
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

pub async fn client(app: &Arc<Mutex<App>>, tx: UnboundedSender<AppDispatch>) {
    let (mut socket, _response) =
        connect(Url::parse("ws://127.0.0.1:9999/inner_client").unwrap()).expect("Can't connect");

    loop {
        let msg = socket.read();

        match msg {
            Ok(l) => {
                match l {
                    tungstenite::Message::Text(s) => {
                        let mut app_guard = app.lock().await;

                        let _ = match parse_raw_trace(&s) {
                            Ok(request) => {
                                app_guard.logs.push(request.to_string());

                                let id = request.id.clone();

                                let cloned_sender = tx.clone();

                                tokio::spawn(async move {
                                    sleep(Duration::from_millis(5000)).await;

                                    cloned_sender
                                        .unbounded_send(AppDispatch::MarkTraceAsTimedOut(id))
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
                    tungstenite::Message::Close(_) => {
                        break;
                    }
                    _ => {
                        panic!()
                    }
                };
            }
            Err(e) => {
                app.lock().await.log(e.to_string());
                break;
            }
        }
    }
}

pub async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    let mut path_rewrite_callback = RequestPath::default();

    let ws_stream = tokio_tungstenite::accept_hdr_async(raw_stream, &mut path_rewrite_callback)
        .await
        .expect("Error during the websocket handshake occurred");

    let path = path_rewrite_callback.uri;

    let (tx, rx) = unbounded();

    peer_map.lock().unwrap().insert(addr, tx);

    // if path != "/inner_client" {
    //     let number_of_connections = peer_map.lock().unwrap().len();
    //
    //     match number_of_connections {
    //         0 => app.lock().await.ws_server_state = WsServerState::Open,
    //         v => app.lock().await.ws_server_state = WsServerState::HasConnections(v),
    //     }
    // }

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

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;
    // tokio::select! {
    //
    // }

    // println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);

    // if path != "/inner_client" {
    //     let number_of_connections = peer_map.lock().unwrap().len();
    //
    //     match number_of_connections {
    //         0 => app.lock().await.ws_server_state = WsServerState::Open,
    //         v => app.lock().await.ws_server_state = WsServerState::HasConnections(v),
    //     }
    //
    //     // match number_of_connections - 1 {
    //     //     0 => app.lock().await.ws_server_state = WsServerState::Open,
    //     //     v => app.lock().await.ws_server_state = WsServerState::HasConnections(v),
    //     // }
    // }
}
