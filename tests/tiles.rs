use std::sync::Arc;
use std::time::Duration;

use backend::client::GameClient;
use backend::generic_result::GenericResult;
use backend::rpc::rpc_types::{PlaceTileInput, RPCClientMessage, RPCServerMessage};
use backend::{run_app, GeneralConfig};
use backend::{test_utils::*, DEFAULT_COLOR};

#[rstest::rstest]
#[tokio::test]
async fn it_should_send_update_to_others() -> GenericResult<()> {
    let config = default_builder().build().unwrap();

    let port = config.port;
    tokio::spawn(async move { run_app(config).await });
    tokio::time::sleep(Duration::from_millis(10)).await;
    let client1 = Arc::new(connect(port).await);
    let client2 = Arc::new(connect(port).await);
    client1
        .send_message(RPCClientMessage::PlaceTile(PlaceTileInput {
            idx: 400,
            tile: 20,
        }))
        .await
        .unwrap();

    loop {
        let msg = client2.receive_message().await.unwrap();
        if let RPCServerMessage::TilePlaced(idx, tile) = msg {
            assert_eq!(idx, 400);
            assert_eq!(tile, 20);
            break;
        }
    }

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
    let submitted_tile = (400u32, 20u8);
    let limited_tile = (401u32, 20u8);
    tokio::spawn(async move { run_app(config).await });
    tokio::time::sleep(Duration::from_millis(10)).await;
    let client1 = Arc::new(connect(port).await);
    client1
        .send_message(RPCClientMessage::PlaceTile(PlaceTileInput {
            idx: submitted_tile.0,
            tile: submitted_tile.1,
        }))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;
    client1
        .send_message(RPCClientMessage::PlaceTile(PlaceTileInput {
            idx: limited_tile.0,
            tile: limited_tile.1,
        }))
        .await
        .unwrap();

    loop {
        let msg = client1.receive_message().await.unwrap();
        if let RPCServerMessage::TilePlaced(idx, tile) = msg {
            assert_eq!(idx, submitted_tile.0);
            assert_eq!(tile, submitted_tile.1);
            break;
        }
    }

    loop {
        let msg = client1.receive_message().await.unwrap();
        if let RPCServerMessage::TilePlaced(idx, tile) = msg {
            assert_eq!(idx, limited_tile.0);
            assert_eq!(tile, DEFAULT_COLOR);

            break;
        }
    }

    Ok(())
}
