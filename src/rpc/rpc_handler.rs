use crate::messages::chat_manager::ChatManager;
use crate::game::Game;
use crate::rpc::rpc_types::RPCServerMessage;
use crate::{generic_result::GenericResult, rpc::rpc_types};

use crate::game::SetTileError;

use crate::messages::chat_manager::SendMessageInput;

use tokio::sync::mpsc::UnboundedSender;

use std::net::IpAddr;
use std::sync::Arc;

pub struct RPCHandler {
    pub game: Arc<Game>,
    pub message_handler: Arc<ChatManager>,
    pub broadcast_tx: UnboundedSender<RPCServerMessage>,
    pub(crate) ip: IpAddr,
    pub(crate) local_sender: UnboundedSender<rpc_types::RPCServerMessage>,
}

impl RPCHandler {
    pub(crate) fn new(
        game: Arc<Game>,
        message_handler: Arc<ChatManager>,
        broadcast_tx: UnboundedSender<RPCServerMessage>,
        ip: IpAddr,
        local_sender: UnboundedSender<rpc_types::RPCServerMessage>,
    ) -> Self {
        Self {
            game,
            message_handler,
            broadcast_tx,
            ip,
            local_sender,
        }
    }
    pub async fn handle_incoming(&self, msg: rpc_types::RPCClientMessage) -> GenericResult<()> {
        match msg {
            rpc_types::RPCClientMessage::SendMessage(input) => {
                self.handle_send_message(input).await?;
            }
            rpc_types::RPCClientMessage::PlaceTile(input) => {
                self.handle_set_tile(input).await?;
            }
        };
        Ok(())
    }

    pub(crate) async fn handle_send_message(
        &self,
        input: rpc_types::RPCSendMessageInput,
    ) -> GenericResult<()> {
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

    pub(crate) async fn handle_set_tile(
        &self,
        input: rpc_types::PlaceTileInput,
    ) -> GenericResult<()> {
        let res = self.game.set_tile(self.ip, input.idx, input.tile).await;
        match res {
            Ok(_) => {
                let message = rpc_types::RPCServerMessage::TilePlaced(input.idx, input.tile);
                self.broadcast_tx.send(message).unwrap();
            }
            Err(SetTileError::RateLimited) => {
                let tile = self.game.get_tile_color(input.idx);
                let message = rpc_types::RPCServerMessage::TilePlaced(input.idx, tile);
                self.local_sender.clone().send(message)?;
            }
            Err(t) => {
                return Err(Box::new(t));
            }
        };
        Ok(())
    }
}
