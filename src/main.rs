use std::time::Duration;

use backend::{chat_manager::ChatManagerConfig, game::GameConfig, run_app, GeneralConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let general_config = GeneralConfig {
        message_handler_config: ChatManagerConfig {
            max_message_length: 400,
            rate_limit_timeout_ms: Duration::from_millis(1),
        },
        game_config: GameConfig {
            height: 2000,
            width: 2000,
            tile_wait_time: Duration::from_secs(10),
        },
        game_file: tokio::fs::read("board.bat").await.ok(),
        port: 3030,
        update_user_count_interval: Duration::from_secs(5),
    };

    run_app(general_config).await
}
