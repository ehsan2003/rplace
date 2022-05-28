#![allow(unused_must_use)]
#[cfg(test)]
#[macro_use]
extern crate rstest;
mod filters;
mod game;
mod rate_limiter_impl;
use core::marker::Send;
mod rate_limiter;
mod users_handler;

mod chat_manager;

pub mod client;
mod dtos;
mod message_censor;
mod message_censor_impl;
#[cfg(test)]
mod mock;
use std::{collections::HashMap, error::Error, net::IpAddr, sync::Arc, time::Duration};

use self::{
    chat_manager::{ChatManager, ChatManagerConfig},
    game::{Game, GameConfig},
    message_censor_impl::MessageCensorerImpl,
};
use dtos::{Clients, ServerMessage};
use futures::{SinkExt, StreamExt};

use rand::{thread_rng, Rng};
use rate_limiter_impl::RateLimiterImpl;
use rgb::RGB8;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    RwLock,
};
use warp::{
    ws::{Message, Ws},
    Filter, Reply,
};
pub type GenericResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct GeneralConfig {
    pub port: u16,
    pub game_width: u32,
    pub game_height: u32,
    pub game_tile_wait_time: Duration,

    pub max_message_length: usize,
    pub message_wait_time: Duration,

    pub update_user_count_interval: Duration,
    pub game_file: Option<Vec<u8>>,

    pub pallette: HashMap<u8, RGB8>,
}

#[derive(Clone)]
pub(crate) struct SharedState {
    pub game: Arc<Game>,
    pub message_handler: Arc<ChatManager>,
    pub broadcast_tx: UnboundedSender<ServerMessage>,
    pub clients: Clients,
}
pub async fn run_app(general_config: GeneralConfig) -> GenericResult<()> {
    let message_handler = create_chat_manager(&general_config);

    let game = create_game(&general_config)?;

    let (broadcast_tx, broadcast_rx) = unbounded_channel::<ServerMessage>();
    let clients: dtos::Clients = Arc::new(RwLock::new(HashMap::new()));
    let shared_state = SharedState {
        game: Arc::new(game),
        message_handler: Arc::new(message_handler),
        broadcast_tx: broadcast_tx.clone(),
        clients: clients.clone(),
    };

    handle_broadcast_messages(shared_state.clients.clone(), broadcast_rx);
    register_user_count_updater(
        general_config.update_user_count_interval,
        shared_state.clients.clone(),
        broadcast_tx.clone(),
    );
    warp::serve(get_routes(&shared_state))
        .run(([127, 0, 0, 1], general_config.port))
        .await;
    Ok(())
}

fn create_game(
    general_config: &GeneralConfig,
) -> Result<Game, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let game_config = GameConfig {
        width: general_config.game_width,
        height: general_config.game_height,
        tile_wait_time: general_config.game_tile_wait_time,
    };
    let game_rate_limit = Arc::new(RateLimiterImpl::new(game_config.tile_wait_time));

    let game = match general_config.game_file.clone() {
        Some(f) => Game::load(f, game_config, game_rate_limit)?,
        None => Game::new(game_config, game_rate_limit),
    };
    Ok(game)
}

fn create_chat_manager(general_config: &GeneralConfig) -> ChatManager {
    let message_censor = Arc::new(MessageCensorerImpl {});
    let rate_limiter = Arc::new(RateLimiterImpl::new(general_config.message_wait_time));
    let config = ChatManagerConfig {
        max_message_length: general_config.max_message_length,
        rate_limit_timeout_ms: general_config.message_wait_time,
    };
    chat_manager::ChatManager::new(config, message_censor, rate_limiter)
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
        .map(|ws: Ws, shared_state: SharedState, ip: IpAddr| {
            ws.on_upgrade(move |w| async move {
                handle_connection(shared_state, ip, w).await;
            })
        });
    let snapshot = warp::path("snapshot")
        .and(shared_state_filter)
        .map(|s: SharedState| s.game.snapshot());
    ws.or(snapshot)
}

async fn handle_connection(shared_state: SharedState, ip: IpAddr, socket: warp::ws::WebSocket) {
    let (local_sender, mut local_receiver) = unbounded_channel();
    let random_id = thread_rng().gen::<u64>();
    {
        shared_state
            .clients
            .write()
            .await
            .insert(random_id, local_sender.clone());
    }
    let (mut websocket_sender, mut websocket_receiver) = socket.split();
    let handler = users_handler::UserHandler::new(shared_state.clone(), ip, local_sender);

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
    shared_state.clients.write().await.remove(&random_id);
}

fn handle_broadcast_messages(
    clients: dtos::Clients,
    mut broadcast_rx: UnboundedReceiver<ServerMessage>,
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
    broadcast_tx: UnboundedSender<ServerMessage>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(delay);
        loop {
            interval.tick().await;
            let clients = clients.read().await;
            let len = clients.len();

            broadcast_tx
                .send(ServerMessage::UpdateUserCount(len as u32))
                .unwrap();
        }
    });
}
