use backend::client::GameClient;
use backend::generic_result::GenericResult;
use backend::rpc::rpc_types::RPCServerMessage;
use backend::test_utils::*;
use backend::{run_app, GeneralConfig};

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;
const COUNT_TO_REACH: usize = 10;

#[rstest::rstest]
#[tokio::test]
async fn it_should_send_correct_count(config: GeneralConfig) -> GenericResult<()> {
    let port = config.port;
    tokio::spawn(async move { run_app(config).await });
    let barrier = Arc::new(Barrier::new(COUNT_TO_REACH));
    let handles = (1..=COUNT_TO_REACH)
        .map(|_| barrier.clone())
        .map(|barrier| {
            tokio::spawn(async move {
                let client = Arc::new(connect(port).await);
                loop_until_sync(client.clone(), barrier.clone()).await;

                loop {
                    let msg = client.receive_message().await.unwrap();
                    if let RPCServerMessage::UpdateUserCount(count) = msg {
                        assert_eq!(count, COUNT_TO_REACH as u32);
                        break;
                    }
                }
                client.disconnect().await.unwrap();
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.await.unwrap()
    }
    Ok(())
}

async fn loop_until_sync(client: Arc<GameClient>, barrier: Arc<Barrier>) {
    let task = tokio::spawn(async move {
        loop {
            let _ = client.receive_message().await.unwrap();
        }
    });
    barrier.wait().await;
    drop(task);
}

#[rstest::rstest]
#[tokio::test]
async fn it_should_decrease_count_after_disconnect(config: GeneralConfig) -> GenericResult<()> {
    let port = config.port;
    let barrier = Arc::new(Barrier::new(COUNT_TO_REACH));
    tokio::spawn(async move { run_app(config).await });
    let handles = (1..=COUNT_TO_REACH)
        .map(|_| barrier.clone())
        .map(|barrier| {
            tokio::spawn(async move {
                let client = Arc::new(connect(port).await);
                loop_until_sync(client.clone(), barrier.clone()).await;

                loop {
                    let msg = client.receive_message().await.unwrap();
                    if let RPCServerMessage::UpdateUserCount(count) = msg {
                        assert_eq!(count, COUNT_TO_REACH as u32);
                        break;
                    }
                }
                client.disconnect().await.unwrap();
            })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        handle.await.unwrap()
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    let client = connect(port).await;

    loop {
        let msg = client.receive_message().await.unwrap();
        if let RPCServerMessage::UpdateUserCount(count) = msg {
            assert_eq!(count, 1);
            break;
        }
    }

    Ok(())
}
