use std::{
    collections::HashMap,
    hash::Hash,
    sync::Mutex,
    time::{Duration, Instant},
};

#[async_trait::async_trait]
pub trait RateLimiter<T: Hash + Eq + Send> {
    async fn is_free(&self, key: &T) -> bool;
    async fn mark_as_limited(&self, key: &T);
}
#[derive(Debug)]
pub struct RateLimiterImpl<T: Hash + Eq> {
    duration: Duration,
    list: Mutex<HashMap<T, Instant>>,
}
impl<T: Hash + Eq> RateLimiterImpl<T> {
    pub fn new(d: Duration) -> Self {
        Self {
            list: (Mutex::new(Default::default())),
            duration: d,
        }
    }
}

impl<T: Hash + Eq+Send> RateLimiterImpl<T> {
    pub async fn is_free(&self, key: &T) -> bool {
        let list = self.list.lock().unwrap();
        let free_at = list.get(key).copied();

        match free_at {
            Some(free_at) => free_at <= Instant::now(),
            None => true,
        }
    }
    pub async fn mark_as_limited(&self, key: T) {
        self.list
            .lock()
            .unwrap()
            .insert(key, Instant::now() + self.duration);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    const DURATION: Duration = Duration::from_millis(10);
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
        std::thread::sleep(DURATION - Duration::from_millis(5));
        assert!(!limiter.is_free(&1).await);
    }

    #[rstest]
    #[tokio::test]
    async fn it_should_work_with_different_keys(limiter: RateLimiterImpl<u32>) {
        limiter.mark_as_limited(1).await;
        assert!(limiter.is_free(&2).await);
    }
}
