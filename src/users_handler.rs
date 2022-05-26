use crate::dtos;

use super::game::SetTileError;

use super::chat_manager::SendMessageInput;

use std::error::Error;

use tokio::sync::mpsc::UnboundedSender;

use std::net::IpAddr;

use super::SharedState;

pub struct UserHandler {
    pub(crate) shared_state: SharedState,
    pub(crate) ip: IpAddr,
    pub(crate) local_sender: UnboundedSender<dtos::ServerMessage>,
}

impl UserHandler {
    pub(crate) fn new(
        shared_state: SharedState,
        ip: IpAddr,
        local_sender: UnboundedSender<dtos::ServerMessage>,
    ) -> Self {
        Self {
            shared_state,
            ip,
            local_sender,
        }
    }
    pub async fn handle_incoming(&self, msg: dtos::WsClientMessage) -> Result<(), Box<dyn Error>> {
        match msg {
            dtos::WsClientMessage::SendMessage(input) => {
                self.handle_send_message(input).await?;
            }
            dtos::WsClientMessage::PlaceTile(input) => {
                self.handle_set_tile(input).await?;
            }
        };
        Ok(())
    }

    pub(crate) async fn handle_send_message(
        &self,
        input: dtos::WsSendMessageInput,
    ) -> Result<(), Box<dyn Error>> {
        let message = SendMessageInput {
            reply_to: input.reply_to,
            text: input.text,
            sender_ip: self.ip,
            channel: input.channel,
            sender_name: input.sender_name,
        };
        let sent_message = self
            .shared_state
            .message_handler
            .handle_message(message)
            .await?;

        let message = dtos::ServerMessage::NewMessage {
            channel: sent_message.channel,
            text: sent_message.text,
            sender_name: sent_message.sender_name,
            reply_to: sent_message.reply_to,
            id: sent_message.id,
        };
        self.shared_state.broadcast_tx.send(message).unwrap();
        Ok(())
    }

    pub(crate) async fn handle_set_tile(
        &self,
        input: dtos::PlaceTileInput,
    ) -> Result<(), Box<dyn Error>> {
        let res = self
            .shared_state
            .game
            .set_tile(self.ip, input.idx, input.tile)
            .await;
        match res {
            Ok(_) => {
                let message = dtos::ServerMessage::TilePlaced(input.idx, input.tile);
                self.shared_state.broadcast_tx.send(message).unwrap();
            }
            Err(SetTileError::RateLimited) => {
                let tile = self.shared_state.game.get_tile_color(input.idx);
                let message = dtos::ServerMessage::TilePlaced(input.idx, tile);
                self.local_sender.clone().send(message)?;
            }
            Err(t) => {
                return Err(Box::new(t));
            }
        };
        Ok(())
    }
}
