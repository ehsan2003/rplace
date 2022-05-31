use super::message_censor::MessageCensorer;
#[derive(Debug)]
pub struct MessageCensorerImpl {}

#[async_trait::async_trait]
impl MessageCensorer for MessageCensorerImpl {
    async fn censor(&self, message: &str) -> String {
        message.to_string()
    }
}
