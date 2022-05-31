use futures::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::{
    generic_result::GenericResult,
    rpc_types::{RPCClientMessage, RPCServerMessage},
};

pub struct GameClient {
    socket: Mutex<WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>>,
}

impl GameClient {
    pub async fn connect(url: &str) -> GenericResult<Self> {
        let (socket, _) = connect_async(url).await?;
        Ok(Self {
            socket: Mutex::new(socket),
        })
    }
    pub async fn send_message(
        &self,
        message: RPCClientMessage,
    ) -> Result<(), Box<dyn std::error::Error>> {
        {
            self.socket
                .lock()
                .await
                .send(Message::text(serde_json::to_string(&message).unwrap()))
        }
        .await?;
        Ok(())
    }
    pub async fn receive_message(&self) -> GenericResult<RPCServerMessage> {
        let message = self.socket.lock().await.next().await.unwrap()?;
        Ok(serde_json::from_str(&message.into_text().unwrap()).unwrap())
    }
}
