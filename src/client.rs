use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};

cfg_if::cfg_if! {
    if #[cfg(target_family = "wasm")] {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen_futures::spawn_local;
        use web_sys::console::log_1 as log;
        use futures_util::stream::SplitStream;
        use futures_util::stream::SplitSink;
        use tokio_tungstenite::WebSocketStream;
        use tokio_tungstenite::MaybeTlsStream;
        use tokio::sync::Mutex;
        use std::sync::Arc;

        #[wasm_bindgen(start)]
        pub async fn start() -> Result<(), JsValue> {
            let ws_stream = connect_to_server("ws://127.0.0.1:8081").await;
            let (write, read) = ws_stream.split();
            let write = Arc::new(Mutex::new(write));
            let read = Arc::new(Mutex::new(read));

            spawn_local(async move {
                loop {
                    if let Some(message) = receive_message(&mut *read.lock().await).await {
                        log(&message.into());
                    }
                }
            });

            Ok(())
        }

        pub async fn connect_to_server(addr: &str) -> WebSocketStream<MaybeTlsStream<TcpStream>> {
            let (ws_stream, _) = connect_async(addr).await.expect("Failed to connect");
            ws_stream
        }

        pub async fn send_message(ws_stream: &mut SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>, message: &str) {
            ws_stream.send(Message::Text(message.to_string())).await.expect("Failed to send message");
        }

        pub async fn receive_message(ws_stream: &mut SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>) -> Option<String> {
            if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
                Some(text)
            } else {
                None
            }
        }
    } else {
        pub async fn connect_to_server(addr: &str) -> tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>> {
            let url = format!("ws://{}", addr);
            let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
            ws_stream
        }

        pub async fn send_message(ws_stream: &mut tokio_tungstenite::WebSocketStream<TcpStream>, message: &str) {
            ws_stream.send(Message::Text(message.to_string())).await.expect("Failed to send message");
        }

        pub async fn receive_message(ws_stream: &mut tokio_tungstenite::WebSocketStream<TcpStream>) -> Option<String> {
            if let Some(Ok(Message::Text(text))) = ws_stream.next().await {
                Some(text)
            } else {
                None
            }
        }
    }
}
