use crate::game::Game;
use crate::messages::chat_manager::ChatManager;
use crate::rpc::rpc_types::RPCServerMessage;
use crate::{generic_result::GenericResult, rpc::rpc_types};

use crate::messages::chat_manager::SendMessageInput;

use tokio::sync::mpsc::UnboundedSender;

use std::net::IpAddr;

use std::sync::Arc;

use super::rpc_types::{RPCSendMessageError, RPCSetTileError};

pub struct RPCHandler {
    pub game: Arc<Game>,
    pub message_handler: Arc<ChatManager>,
    pub broadcast_tx: UnboundedSender<RPCServerMessage>,
    pub(crate) ip: IpAddr,
    pub(crate) local_sender: UnboundedSender<rpc_types::RPCServerMessage>,
}

impl RPCHandler {
    pub(crate) async fn handle_set_tile(
        &self,
        input: &rpc_types::PlaceTileInput,
    ) -> Result<(), RPCSetTileError> {
        let res = self.game.set_tile(self.ip, input.idx, input.tile).await?;
        let message = rpc_types::RPCServerMessage::TilePlaced(input.idx, input.tile);
        self.broadcast_tx.send(message).unwrap();
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
