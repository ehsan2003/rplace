#![allow(unused_must_use)]
#[cfg(test)]
#[macro_use]
extern crate rstest;
mod filters;
mod game;

pub mod client;

mod messages;
#[cfg(test)]
mod mock;
mod rate_limit;
pub mod rpc;

use std::{collections::HashMap, net::IpAddr, ops::Deref, sync::Arc, time::Duration};

use self::{
    game::{Game, GameConfig},
    messages::chat_manager::{ChatManager, ChatManagerConfig},
    messages::message_censor_impl::MessageCensorerImpl,
};
use futures::{SinkExt, StreamExt};
use rpc::{
    rpc_handler::RPCHandler,
    rpc_types::{self, RPCServerMessage, RPCSetTileError},
};

use rand::{thread_rng, Rng};
use rate_limit::rate_limiter_impl::RateLimiterImpl;
use rgb::RGB8;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    RwLock,
};
use warp::{
    ws::{Message, Ws},
    Filter, Reply,
};
pub mod generic_result;
pub type Clients = Arc<RwLock<HashMap<u64, UnboundedSender<RPCServerMessage>>>>;

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
    pub broadcast_tx: UnboundedSender<RPCServerMessage>,
    pub clients: Clients,
}
pub async fn run_app(general_config: GeneralConfig) -> generic_result::GenericResult<()> {
    let message_handler = create_chat_manager(&general_config);

    let game = create_game(&general_config)?;

    let (broadcast_tx, broadcast_rx) = unbounded_channel::<RPCServerMessage>();
    let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
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

    warp::serve(get_routes(&shared_state, Arc::new(general_config.pallette)))
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
    {
        let l = game_rate_limit.clone();
        tokio::spawn(async move {
            l.run_garbage_collector(
                Duration::from_secs(60),
                tokio::sync::broadcast::channel(1).1,
            )
            .await;
        });
    }
    let game = match general_config.game_file.clone() {
        Some(f) => Game::load(f, game_config, game_rate_limit)?,
        None => Game::new(game_config, game_rate_limit),
    };
    Ok(game)
}

fn create_chat_manager(general_config: &GeneralConfig) -> ChatManager {
    let message_censor = Arc::new(MessageCensorerImpl {});
    let rate_limiter = Arc::new(RateLimiterImpl::new(general_config.message_wait_time));
    {
        let l = rate_limiter.clone();
        tokio::spawn(async move {
            l.run_garbage_collector(
                Duration::from_secs(60),
                tokio::sync::broadcast::channel(1).1,
            )
            .await;
        });
    }
    let config = ChatManagerConfig {
        max_message_length: general_config.max_message_length,
        rate_limit_timeout_ms: general_config.message_wait_time,
    };
    ChatManager::new(config, message_censor, rate_limiter)
}

fn get_routes(
    shared_state: &SharedState,
    pallette: Arc<HashMap<u8, RGB8>>,
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
    let pallette = warp::path("pallette")
        .map(move || pallette.clone())
        .map(|p: Arc<HashMap<u8, RGB8>>| warp::reply::json(p.deref()));
    ws.or(snapshot).or(pallette)
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
    let handler = RPCHandler::new(
        shared_state.game.clone(),
        shared_state.message_handler.clone(),
        shared_state.broadcast_tx.clone(),
        ip,
    );

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

        match client_message {
            rpc_types::RPCClientMessage::SendMessage(input) => {
                handler.handle_send_message(input).await;
            }
            rpc_types::RPCClientMessage::PlaceTile(input) => {
                let res = handler.handle_set_tile(&input).await;
                if res.is_err() {
                    let tile = shared_state.game.get_tile_color(input.idx);

                    if let Some(tile) = tile {
                        let message = rpc_types::RPCServerMessage::TilePlaced(input.idx, tile);
                        local_sender.send(message);
                    }
                };
            }
        };
    }
    shared_state.clients.write().await.remove(&random_id);
}

fn handle_broadcast_messages(
    clients: Clients,
    mut broadcast_rx: UnboundedReceiver<RPCServerMessage>,
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
    clients: Clients,
    broadcast_tx: UnboundedSender<RPCServerMessage>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(delay);
        loop {
            interval.tick().await;
            let clients = clients.read().await;
            let len = clients.len();

            broadcast_tx
                .send(RPCServerMessage::UpdateUserCount(len as u32))
                .unwrap();
        }
    });
}
