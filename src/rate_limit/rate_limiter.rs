use std::{hash::Hash, sync::Arc};

#[async_trait::async_trait]
pub trait RateLimiter<T: Hash + Eq + Send + Sync>: Send + Sync {
    async fn is_free(&self, key: &T) -> bool;
    async fn mark_as_limited(&self, key: T);
}
pub type SharedRateLimiter<T> = Arc<dyn RateLimiter<T>>;
