use crate::message_censor::MessageCensorer;

pub struct MessageCensorerImpl {}

#[async_trait::async_trait]
impl MessageCensorer for MessageCensorerImpl {
    async fn censor(&self, message: &str) -> String {
        message.to_string()
    }
}
