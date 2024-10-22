use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};

async fn handle_connection(raw_stream: tokio::net::TcpStream) {
    let ws_stream = accept_async(raw_stream).await.expect("Error during the websocket handshake occurred");
    let (mut write, mut read) = ws_stream.split();

    while let Some(message) = read.next().await {
        let message = message.expect("Error reading message");
        if let Message::Text(text) = message {
            write.send(Message::Text(text)).await.expect("Error sending message");
        }
    }
}

pub async fn start_server(addr: &str) {
    let listener = TcpListener::bind(addr).await.expect("Failed to bind");
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream));
    }
}
