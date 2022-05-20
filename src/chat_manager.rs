use std::{
    net::IpAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

use crate::{message_censor::MessageCensorer, rate_limiter::RateLimiter};

pub struct ChatManagerConfig {
    pub max_message_length: usize,
    pub rate_limit_timeout_ms: Duration,
}
pub struct ChatManager {
    max_message_length: usize,
    rate_limiter: RateLimiter<IpAddr>,
    censorer: Arc<dyn MessageCensorer + Send + Sync>,
    id_counter: AtomicU64,
}
pub struct SendMessageInput {
    text: String,
    sender_id: String,
    sender_ip: IpAddr,
    reply_to: Option<u64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatError {
    RateLimited,
    InvalidMessageLength,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct SentMessage {
    pub text: String,
    pub sender_id: String,
    pub id: u64,
    pub reply_to: Option<u64>,
}
impl ChatManager {
    pub fn new(
        config: ChatManagerConfig,
        censorer: Arc<dyn MessageCensorer + Send + Sync>,
    ) -> Self {
        ChatManager {
            max_message_length: config.max_message_length,
            censorer,
            rate_limiter: RateLimiter::new(config.rate_limit_timeout_ms),
            id_counter: AtomicU64::new(0),
        }
    }
    pub async fn handle_message(
        &self,
        message: SendMessageInput,
    ) -> Result<SentMessage, ChatError> {
        let text = self.censorer.censor(message.text.trim()).await;

        if text.len() > self.max_message_length || text.len() < 1 {
            return Err(ChatError::InvalidMessageLength);
        }

        if !self.rate_limiter.is_free(&message.sender_ip) {
            return Err(ChatError::RateLimited);
        }
        self.rate_limiter.mark_as_limited(message.sender_ip);
        Ok(SentMessage {
            text,
            sender_id: message.sender_id,
            id: self.id_counter.fetch_add(1, Ordering::Relaxed),
            reply_to: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use rand::prelude::*;
    use std::{
        net::Ipv6Addr,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use crate::mock::Mock;

    use super::*;
    const MAX_MESSAGE_LENGTH: usize = 400;
    const RATE_LIMIT_TIMEOUT: Duration = Duration::from_millis(1);
    #[derive(Default)]
    pub struct MockMessageCensorer {
        pub censor: Mock<String, String>,
    }
    impl MockMessageCensorer {
        pub fn new(mock: Mock<String, String>) -> Self {
            MockMessageCensorer { censor: mock }
        }
    }

    #[async_trait::async_trait]
    impl MessageCensorer for MockMessageCensorer {
        async fn censor(&self, message: &str) -> String {
            {
                self.censor.call(message.to_string())
            }
        }
    }
    #[fixture]
    fn censorer_mock() -> Arc<MockMessageCensorer> {
        Arc::new(MockMessageCensorer::new(Mock::new().fake(|s| s)))
    }
    #[fixture]
    fn manager(censorer_mock: Arc<MockMessageCensorer>) -> ChatManager {
        ChatManager::new(
            ChatManagerConfig {
                max_message_length: MAX_MESSAGE_LENGTH,
                rate_limit_timeout_ms: RATE_LIMIT_TIMEOUT,
            },
            censorer_mock,
        )
    }
    #[fixture]
    fn manager_with_censorer(
        censorer_mock: Arc<MockMessageCensorer>,
    ) -> (ChatManager, Arc<MockMessageCensorer>) {
        (
            ChatManager::new(
                ChatManagerConfig {
                    max_message_length: MAX_MESSAGE_LENGTH,
                    rate_limit_timeout_ms: RATE_LIMIT_TIMEOUT,
                },
                censorer_mock.clone(),
            ),
            censorer_mock.clone(),
        )
    }
    fn fake_user(msg: impl Into<String>) -> SendMessageInput {
        SendMessageInput {
            text: msg.into(),
            sender_id: rand::random::<u64>().to_string(),
            sender_ip: random_ip(),
            reply_to: None,
        }
    }

    fn random_ip() -> std::net::IpAddr {
        let mut rng = rand::thread_rng();

        IpAddr::V6(Ipv6Addr::new(
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
            rng.gen::<u16>(),
        ))
    }
    #[rstest]
    #[case(format!("   {}   ", "h".repeat(9)))]
    #[case(format!("\n\n{}\n\n", "h".repeat(9)))]
    #[tokio::test]
    async fn it_must_not_refuse_massage_if_length_is_shorter_after_sanitization(
        manager: ChatManager,
        #[case] message: String,
    ) {
        let result = manager.handle_message(fake_user(message)).await;
        assert!(result.is_ok())
    }

    #[rstest]
    #[tokio::test]
    async fn it_must_call_censorer_with_trimmed_message(
        manager_with_censorer: (ChatManager, Arc<MockMessageCensorer>),
    ) {
        let (manager, censorer_mock) = manager_with_censorer;
        manager.handle_message(fake_user(" hello ")).await.unwrap();
        censorer_mock.censor.assert_first_call("hello".into());
    }

    #[rstest]
    #[case("c".repeat(MAX_MESSAGE_LENGTH + 1))]
    #[case("")]
    #[tokio::test]
    async fn it_should_match_length_for_censored_text(
        mut manager: ChatManager,
        #[case] censor_output: String,
    ) {
        manager.censorer = Arc::new(MockMessageCensorer::new(
            Mock::new().returning(censor_output),
        ));

        let err = manager
            .handle_message(fake_user("not-important"))
            .await
            .unwrap_err();
        assert_eq!(err, ChatError::InvalidMessageLength);
    }

    /// a huge test for checking whether the message id is not duplicated
    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn it_must_generate_new_id_for_each_message(manager: ChatManager) {
        let manager = Arc::new(manager);
        let id_set = Arc::new(Mutex::new(std::collections::HashSet::<u64>::new()));

        let handles = (0..100000)
            .map(|_| {
                let manager = manager.clone();
                let set = id_set.clone();
                tokio::spawn(async move {
                    let r = manager.handle_message(fake_user("hello")).await.unwrap();

                    let mut set = set.lock().unwrap();
                    if set.contains(&r.id) {
                        panic!("id is not unique");
                    } else {
                        set.insert(r.id);
                    }
                })
            })
            .collect::<Vec<_>>();
        for h in handles {
            h.await.unwrap();
        }
    }

    #[rstest]
    #[tokio::test]
    async fn it_must_rate_limit_requests(manager: ChatManager) {
        let ip = random_ip();
        manager
            .handle_message(SendMessageInput {
                reply_to: None,
                sender_id: "1".to_string(),
                sender_ip: ip,
                text: "hello".to_string(),
            })
            .await
            .unwrap();

        let err = manager
            .handle_message(SendMessageInput {
                reply_to: None,
                sender_id: "2".to_string(),
                sender_ip: ip,
                text: "hello2".to_string(),
            })
            .await
            .unwrap_err();

        assert_eq!(err, ChatError::RateLimited);
    }

    #[rstest]
    #[tokio::test]
    async fn it_must_return_censored_text(mut manager: ChatManager) {
        let censorer = Arc::new(MockMessageCensorer::new(
            Mock::new().returning("censored".into()),
        ));
        manager.censorer = censorer.clone();
        let result = manager.handle_message(fake_user("hello")).await.unwrap();
        assert_eq!(result.text, "censored");
    }
}
