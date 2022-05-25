#![allow(unused_must_use)]
#[cfg(test)]
#[macro_use]
extern crate rstest;
mod filters;
pub mod game;
pub mod rate_limiter;

pub mod chat_manager;

pub mod message_censor;
pub mod message_censor_impl;
pub mod mock;

use std::{collections::HashMap, error::Error, net::IpAddr, sync::Arc, time::Duration};

use self::{
    chat_manager::{ChatManager, ChatManagerConfig, SendMessageInput},
    game::{Game, GameConfig, SetTileError},
    message_censor_impl::MessageCensorerImpl,
};
use futures::{SinkExt, StreamExt};
use rand::{thread_rng, Rng};
use rate_limiter::RateLimiterImpl;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    RwLock,
};
use warp::{
    ws::{Message, Ws},
    Filter, Reply,
};

mod dtos;

#[derive(Debug, Clone)]
pub struct GeneralConfig {
    pub port: u16,
    pub game_config: GameConfig,
    pub message_handler_config: ChatManagerConfig,
    pub update_user_count_interval: Duration,
    pub game_file: Option<Vec<u8>>,
}

#[derive(Clone)]
pub struct SharedState {
    pub game: Arc<Game>,
    pub message_handler: Arc<ChatManager>,
    pub broadcast_tx: UnboundedSender<dtos::ServerMessage>,
    pub clients: Arc<RwLock<HashMap<u64, UnboundedSender<dtos::ServerMessage>>>>,
}
pub async fn run_app(general_config: GeneralConfig) -> Result<(), Box<dyn Error>> {
    let GeneralConfig {
        message_handler_config,
        game_config,
        game_file,
        port,
        update_user_count_interval,
    } = general_config;

    let message_censor = Arc::new(MessageCensorerImpl {});
    let message_handler_rate_limiter = Arc::new(RateLimiterImpl::new(
        message_handler_config.rate_limit_timeout_ms,
    ));
    let message_handler = chat_manager::ChatManager::new(
        message_handler_config,
        message_censor.clone(),
        message_handler_rate_limiter,
    );
    let game_rate_limit = Arc::new(RateLimiterImpl::new(game_config.tile_wait_time));
    let game = match game_file {
        Some(f) => Game::load(f, game_config, game_rate_limit)?,
        None => Game::new(game_config, game_rate_limit),
    };
    let (broadcast_tx, broadcast_rx) = unbounded_channel::<dtos::ServerMessage>();
    let clients: dtos::Clients = Arc::new(RwLock::new(HashMap::new()));
    let shared_state = SharedState {
        game: Arc::new(game),
        message_handler: Arc::new(message_handler),
        broadcast_tx: broadcast_tx.clone(),
        clients: clients.clone(),
    };

    handle_broadcast_messages(shared_state.clients.clone(), broadcast_rx);
    register_user_count_updater(
        update_user_count_interval,
        shared_state.clients.clone(),
        broadcast_tx.clone(),
    );
    warp::serve(get_routes(&shared_state))
        .run(([127, 0, 0, 1], port))
        .await;
    Ok(())
}

fn get_routes(
    shared_state: &SharedState,
) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
    let shared_state_filter = {
        let t = shared_state.clone();
        warp::any().map(move || t.clone())
    };
    let ws = warp::path("ws")
        .and(warp::ws())
        .and(shared_state_filter.clone())
        .and(filters::ip_extractor())
        .map(|ws: Ws, ss: SharedState, ip: IpAddr| {
            ws.on_upgrade(move |w| async move {
                handle_connection(ss, ip, w).await;
            })
        });
    let file_getter = warp::path("file")
        .and(shared_state_filter)
        .map(|s: SharedState| s.game.snapshot());
    ws.or(file_getter)
}

async fn handle_connection(ss: SharedState, ip: IpAddr, socket: warp::ws::WebSocket) {
    let (local_sender, mut local_receiver) = unbounded_channel();
    let random_id = thread_rng().gen::<u64>();
    {
        ss.clients
            .write()
            .await
            .insert(random_id, local_sender.clone());
    }
    let (mut websocket_sender, mut websocket_receiver) = socket.split();
    let handler = UserHandler::new(ss.clone(), ip, local_sender);

    tokio::spawn(async move {
        while let Some(msg) = local_receiver.recv().await {
            if let Err(e) = websocket_sender
                .send(Message::text(serde_json::to_string(&msg).unwrap()))
                .await
            {
                println!("Error sending message: {}", e);
                break;
            }
        }
    });
    while let Some(msg) = websocket_receiver.next().await {
        let client_message = match msg {
            Ok(msg) if msg.is_text() => match serde_json::from_slice(msg.as_bytes()) {
                Ok(m) => m,
                Err(_) => continue,
            },
            Err(_) => {
                eprintln!("Error");
                break;
            }
            _ => continue,
        };
        handler.handle_incoming(client_message).await;
    }
    ss.clients.write().await.remove(&random_id);
}
pub struct UserHandler {
    shared_state: SharedState,
    ip: IpAddr,
    local_sender: UnboundedSender<dtos::ServerMessage>,
}

impl UserHandler {
    pub fn new(
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

    async fn handle_send_message(
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

    async fn handle_set_tile(&self, input: dtos::PlaceTileInput) -> Result<(), Box<dyn Error>> {
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

fn handle_broadcast_messages(
    clients: dtos::Clients,
    mut broadcast_rx: UnboundedReceiver<dtos::ServerMessage>,
) {
    tokio::spawn(async move {
        while let Some(msg) = broadcast_rx.recv().await {
            let clients = clients.read().await;
            for (_, tx) in clients.iter() {
                tx.send(msg.clone());
            }
        }
    });
}

fn register_user_count_updater(
    delay: Duration,
    clients: dtos::Clients,
    broadcast_tx: UnboundedSender<dtos::ServerMessage>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(delay);
        loop {
            interval.tick().await;
            let clients = clients.read().await;
            let len = clients.len();

            broadcast_tx
                .send(dtos::ServerMessage::UpdateUserCount(len as u32))
                .unwrap();
        }
    });
}
