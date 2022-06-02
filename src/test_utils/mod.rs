use std::time::Duration;

use rgb::RGB8;

use crate::{client::GameClient, GeneralConfig, GeneralConfigBuilder};

pub fn default_builder() -> GeneralConfigBuilder {
    let mut builder = GeneralConfigBuilder::default();
    let port = portpicker::pick_unused_port().unwrap();

    builder
        .port(port)
        .game_width(2000)
        .game_height(2000)
        .game_tile_wait_time(Duration::from_millis(100))
        .max_message_length(4000)
        .message_wait_time(Duration::from_millis(100))
        .update_user_count_interval(Duration::from_millis(10000))
        .game_file(None)
        .pallette(
            vec![
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
                    (f >> 16 & 0xff) as u8,
                    (f >> 8 & 0xff) as u8,
                    (f & 0xff) as u8,
                )
            })
            .enumerate()
            .map(|(i, f)| (i as u8, f))
            .collect(),
        );
    builder
}



pub async fn connect(port: u16) -> GameClient {
    GameClient::connect(&format!("ws://127.0.0.1:{port}/ws"))
        .await
        .unwrap()
}
