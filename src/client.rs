use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::protocol::Message;
use futures_util::{StreamExt, SinkExt};
use futures_util::stream::StreamExt;
use futures_util::sink::SinkExt;

pub async fn connect_to_server(addr: &str) -> tokio_tungstenite::WebSocketStream<TcpStream> {
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
