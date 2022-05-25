use std::time::Duration;

use backend::{run_app, GeneralConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let general_config = GeneralConfig {
        max_message_length: 400,
        message_wait_time: Duration::from_millis(1),

        game_height: 2000,
        game_width: 2000,
        game_tile_wait_time: Duration::from_secs(10),
        game_file: tokio::fs::read("board.bat").await.ok(),

        port: 3030,
        update_user_count_interval: Duration::from_secs(5),
    };

    run_app(general_config).await
}
