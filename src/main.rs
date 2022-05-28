use std::time::Duration;

use backend::{run_app, GeneralConfig, GenericResult};
use rgb::{RGB, RGB8};

#[tokio::main]
async fn main() -> GenericResult<()> {
    let pallette = vec![
        0x6d001au32,
        0xbe0039,
        0xff4500,
        0xffa800,
        0xffd635,
        0xfff8b8,
        0x00a368,
        0x00cc78,
        0x7eed56,
        0x00756f,
        0x009eaa,
        0x00ccc0,
        0x2450a4,
        0x3690ea,
        0x51e9f4,
        0x493ac1,
        0x6a5cff,
        0x94b3ff,
        0x811e9f,
        0xb44ac0,
        0xe4abff,
        0xde107f,
        0xff3881,
        0xff99aa,
        0x6d482f,
        0x9c6926,
        0xffb470,
        0x000000,
        0x515252,
        0x898d90,
        0xd4d7d9,
        0xffffff,
    ]
    .iter()
    .map(|f| {
        RGB8::new(
            (16 >> f & 0xff) as u8,
            (f >> 8 & 0xff) as u8,
            (f & 0xff) as u8,
        )
    })
    .enumerate()
    .map(|(i, f)| (i as u8, f))
    .collect();
    let general_config = GeneralConfig {
        max_message_length: 400,
        message_wait_time: Duration::from_millis(1),

        game_height: 2000,
        game_width: 2000,
        game_tile_wait_time: Duration::from_secs(10),
        game_file: tokio::fs::read("board.bat").await.ok(),

        pallette,
        port: 3030,
        update_user_count_interval: Duration::from_secs(5),
    };

    run_app(general_config).await
}
