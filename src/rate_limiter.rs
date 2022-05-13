use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
#[derive(Clone, Debug)]
pub struct RateLimiter<T: Hash + Eq> {
    duration: Duration,
    list: Arc<Mutex<HashMap<T, Instant>>>,
}
impl<T: Hash + Eq> RateLimiter<T> {
    pub fn new(d: Duration) -> Self {
        Self {
            list: Arc::new(Mutex::new(Default::default())),
            duration: d,
        }
    }
    pub fn is_free(&self, key: &T) -> bool {
        let list = self.list.lock().unwrap();
        let free_at = list.get(key).copied();

        match free_at {
            Some(free_at) => !(free_at > Instant::now()),
            None => true,
        }
    }
    pub fn mark_as_limited(&self, key: T) {
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
    fn limiter() -> RateLimiter<u32> {
        RateLimiter::new(DURATION)
    }

    #[rstest]
    fn it_must_free_at_first_call(limiter: RateLimiter<u32>) {
        assert!(limiter.is_free(&1));
    }

    #[rstest]
    fn it_must_not_be_free_after_marked_as_limited(limiter: RateLimiter<u32>) {
        limiter.mark_as_limited(1);
        assert!(!limiter.is_free(&1));
    }

    #[rstest]
    fn it_must_be_free_after_time_passed(limiter: RateLimiter<u32>) {
        limiter.mark_as_limited(1);
        std::thread::sleep(DURATION + Duration::from_millis(1));
        assert!(limiter.is_free(&1));
    }

    #[rstest]
    fn it_must_not_be_free_before_time_passed(limiter: RateLimiter<u32>) {
        limiter.mark_as_limited(1);
        std::thread::sleep(DURATION - Duration::from_millis(1));
        assert!(!limiter.is_free(&1));
    }

    #[rstest]
    fn it_should_work_with_different_keys(limiter: RateLimiter<u32>) {
        limiter.mark_as_limited(1);
        assert!(limiter.is_free(&2));
    }

    #[rstest]
    fn clones_must_share_work(limiter: RateLimiter<u32>) {
        let limiter2 = limiter.clone();
        limiter.mark_as_limited(1);
        assert!(!limiter2.is_free(&1));
    }
}
