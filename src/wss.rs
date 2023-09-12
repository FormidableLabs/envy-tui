use futures_util::{StreamExt, TryStreamExt};
use tokio_tungstenite::connect_async;

pub async fn connect() {
    let connect_addr = "ws://localhost:9002";
    let url = url::Url::parse(&connect_addr).unwrap();

    match connect_async(url).await {
        Ok(conn) => {
          println!("connected");
          let (ws_stream, _) = conn;
          let (_write, read) = ws_stream.split();
          const MAX_CONCURRENT_JUMPERS: usize = 100;
          let read_messages = read.try_for_each_concurrent(MAX_CONCURRENT_JUMPERS, |msg| async move {
              println!("message received: {}", msg);
              Ok(())
          });

          let _ = read_messages.await;
        },
        Err(e) => println!("failed to connect: {}", e),
    }
}
