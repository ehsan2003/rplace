use std::{time::{Duration, Instant}, collections::HashMap, sync::RwLock, hash::Hash};

use crate::rate_limiter::RateLimiter;

#[derive(Debug)]
pub struct RateLimiterImpl<T: Hash + Eq + Send + Sync> {
    duration: Duration,
    list: RwLock<HashMap<T, Instant>>,
}
impl<T: Hash + Eq + Send + Sync> RateLimiterImpl<T> {
    pub fn new(d: Duration) -> Self {
        Self {
            list: (RwLock::new(Default::default())),
            duration: d,
        }
    }
}
#[async_trait::async_trait]
impl<T: Hash + Eq + Send + Sync> RateLimiter<T> for RateLimiterImpl<T> {
    async fn is_free(&self, key: &T) -> bool {
        let list = self.list.read().unwrap();
        let free_at = list.get(key).copied();

        match free_at {
            Some(free_at) => free_at <= Instant::now(),
            None => true,
        }
    }
    async fn mark_as_limited(&self, key: T) {
        self.list
            .write()
            .unwrap()
            .insert(key, Instant::now() + self.duration);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    const DURATION: Duration = Duration::from_millis(100);
    #[fixture]
    fn limiter() -> RateLimiterImpl<u32> {
        RateLimiterImpl::new(DURATION)
    }

    #[rstest]
    #[tokio::test]
    async fn it_must_free_at_first_call(limiter: RateLimiterImpl<u32>) {
        assert!(limiter.is_free(&1).await);
    }

    #[rstest]
    #[tokio::test]
    async fn it_must_not_be_free_after_marked_as_limited(limiter: RateLimiterImpl<u32>) {
        limiter.mark_as_limited(1).await;
        assert!(!limiter.is_free(&1).await);
    }

    #[rstest]
    #[tokio::test]
    async fn it_must_be_free_after_time_passed(limiter: RateLimiterImpl<u32>) {
        limiter.mark_as_limited(1).await;
        std::thread::sleep(DURATION + Duration::from_millis(1));
        assert!(limiter.is_free(&1).await);
    }

    #[rstest]
    #[tokio::test]
    async fn it_must_not_be_free_before_time_passed(limiter: RateLimiterImpl<u32>) {
        limiter.mark_as_limited(1).await;
        std::thread::sleep(DURATION - Duration::from_millis(50));
        assert!(!limiter.is_free(&1).await);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_work_with_different_keys(limiter: RateLimiterImpl<u32>) {
        limiter.mark_as_limited(1).await;
        assert!(limiter.is_free(&2).await);
    }
}
