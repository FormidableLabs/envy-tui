use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, StreamExt, TryStreamExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tungstenite::connect;
use url::Url;

use crate::app::{ActiveBlock, App};

use tungstenite::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<std::sync::Mutex<HashMap<SocketAddr, Tx>>>;

pub async fn client(app: &Arc<Mutex<App>>) {
    let (mut socket, _response) =
        connect(Url::parse("ws://127.0.0.1:9999").unwrap()).expect("Can't connect");

    loop {
        let msg = socket.read().expect("Error reading message");
        println!("after");

        let msg = match msg {
            tungstenite::Message::Text(s) => s,
            _ => {
                println!("tt");

                panic!()
            }
        };
        println!("after!!");
        let mut ss = app.lock().await;
        ss.active_block = ActiveBlock::ResponseDetails;

        println!("!!!!!!!!!!!!!!!!!!!!!");
        // let d = match ss.await {
        //     // Ok(mut g) => {
        //     //     g.active_block = ActiveBlock::ResponseDetails;
        //     //     println!("!!!!!!!!!!!!!!!!!!!!!");
        //     //
        //     //     g
        //     // }
        //     // Err(g) => g.into_inner(),
        // };
        //
        // println!("Received: {}", msg);
        // println!("Received:");
        // std::mem::drop(ss);
    }

    // println!("here");
    // socket.close(None);
}

pub async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    println!("Incoming TCP connection from: {}", addr);

    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();

    peer_map.lock().unwrap().insert(addr, tx);

    let (outgoing, incoming) = ws_stream.split();

    let broadcast_incoming = incoming.try_for_each(|msg| {
        println!(
            "Received a message from {}: {}",
            addr,
            msg.to_text().unwrap()
        );

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
    //
    // pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}
