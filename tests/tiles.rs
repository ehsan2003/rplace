use std::sync::Arc;
use std::time::Duration;

use backend::client::GameClient;
use backend::generic_result::GenericResult;
use backend::rpc::rpc_types::{RPCClientMessage, RPCServerMessage, Tile};
use backend::{run_app, GeneralConfig};
use backend::{test_utils::*, DEFAULT_COLOR};

#[rstest::rstest]
#[tokio::test]
async fn it_should_send_update_to_others() -> GenericResult<()> {
    let config = default_builder().build().unwrap();

    let port = config.port;
    tokio::spawn(async move { run_app(config).await });
    tokio::time::sleep(Duration::from_millis(10)).await;
    let sender = Arc::new(connect(port).await);
    let receiver = Arc::new(connect(port).await);
    let submitted_tile = Tile { idx: 400, tile: 20 };
    sender
        .send_message(RPCClientMessage::PlaceTile(submitted_tile.clone()))
        .await
        .unwrap();

    loop {
        let msg = receiver.receive_message().await.unwrap();
        if let RPCServerMessage::TilePlaced(tile) = msg {
            assert_eq!(tile, submitted_tile);
            break;
        }
    }

    loop {
        let msg = sender.receive_message().await.unwrap();
        if let RPCServerMessage::TilePlaced(tile) = msg {
            assert_eq!(tile, submitted_tile);
            break;
        }
    }
    dbg!("end");
    Ok(())
}

#[rstest::rstest]
#[tokio::test]
async fn it_should_rate_limit() -> GenericResult<()> {
    let config = default_builder()
        .game_tile_wait_time(Duration::from_secs(10))
        .build()
        .unwrap();

    let port = config.port;
    let submitted_tile = Tile::new(400u32, 20u8);
    let limited_tile = Tile::new(401u32, 20u8);
    tokio::spawn(async move { run_app(config).await });
    tokio::time::sleep(Duration::from_millis(10)).await;
    let client1 = Arc::new(connect(port).await);
    client1
        .send_message(RPCClientMessage::PlaceTile(submitted_tile.clone()))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(1000)).await;
    client1
        .send_message(RPCClientMessage::PlaceTile(limited_tile.clone()))
        .await
        .unwrap();

    loop {
        let msg = client1.receive_message().await.unwrap();
        if let RPCServerMessage::TilePlaced(tile) = msg {
            assert_eq!(tile, submitted_tile);
            break;
        }
    }
    dbg!("REACHED");
    loop {
        let msg = client1.receive_message().await.unwrap();
        if let RPCServerMessage::TilePlaced(tile) = msg {
            assert_eq!(tile, Tile::new(limited_tile.idx, DEFAULT_COLOR));

            break;
        }
    }

    Ok(())
}
