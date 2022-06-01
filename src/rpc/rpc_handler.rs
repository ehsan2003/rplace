use crate::game::Game;
use crate::messages::chat_manager::ChatManager;
use crate::rpc::rpc_types;
use crate::rpc::rpc_types::RPCServerMessage;

use crate::messages::chat_manager::SendMessageInput;

use tokio::sync::mpsc::UnboundedSender;

use std::net::IpAddr;

use std::sync::Arc;

use super::rpc_types::{RPCSendMessageError, RPCSetTileError};

pub struct RPCHandler {
    game: Arc<Game>,
    message_handler: Arc<ChatManager>,
    broadcast_tx: UnboundedSender<RPCServerMessage>,
    ip: IpAddr,
}
impl RPCHandler {
    pub fn new(
        game: Arc<Game>,
        message_handler: Arc<ChatManager>,
        broadcast_tx: UnboundedSender<RPCServerMessage>,
        ip: IpAddr,
    ) -> Self {
        Self {
            game,
            message_handler,
            broadcast_tx,
            ip,
        }
    }

    pub(crate) async fn handle_set_tile(
        &self,
        input: &rpc_types::PlaceTileInput,
    ) -> Result<(), RPCSetTileError> {
        let current_color = self.game.get_tile_color(input.idx);
        if let Some(current_color) = current_color {
            if current_color == input.tile {
                return Ok(());
            }
        }
        let res = self.game.set_tile(self.ip, input.idx, input.tile).await?;
        let rpc_message = rpc_types::RPCServerMessage::TilePlaced(input.idx, input.tile);
        self.broadcast_tx.send(rpc_message).unwrap();
        Ok(res)
    }

    pub(crate) async fn handle_send_message(
        &self,
        input: rpc_types::RPCSendMessageInput,
    ) -> Result<(), RPCSendMessageError> {
        let message = SendMessageInput {
            reply_to: input.reply_to,
            text: input.text,
            sender_ip: self.ip,
            channel: input.channel,
            sender_name: input.sender_name,
        };
        let sent_message = self.message_handler.handle_message(message).await?;

        let message = rpc_types::RPCServerMessage::NewMessage {
            channel: sent_message.channel,
            text: sent_message.text,
            sender_name: sent_message.sender_name,
            reply_to: sent_message.reply_to,
            id: sent_message.id,
        };
        self.broadcast_tx.send(message).unwrap();
        Ok(())
    }
}
