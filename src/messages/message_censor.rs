#[async_trait::async_trait]
pub trait MessageCensorer {
    async fn censor(&self, message: &str) -> String;
}
