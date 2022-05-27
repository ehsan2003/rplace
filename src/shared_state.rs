use crate::chat_manager::ChatManager;
use crate::dtos::{Clients, ServerMessage};
use crate::game::Game;

use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SharedState {
    pub game: Arc<Game>,
    pub message_handler: Arc<ChatManager>,
    pub broadcast_tx: tokio::sync::mpsc::UnboundedSender<ServerMessage>,
    pub clients: Clients,
    pub shutdown_tx: tokio::sync::broadcast::Sender<()>,
    pub shutdown_ack_drop: tokio::sync::mpsc::Sender<()>,
}
