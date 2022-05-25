use std::{
    collections::HashMap,
    hash::Hash,
    sync::Mutex,
    time::{Duration, Instant},
};
#[derive(Debug)]
pub struct RateLimiter<T: Hash + Eq> {
    duration: Duration,
    list: Mutex<HashMap<T, Instant>>,
}
impl<T: Hash + Eq> RateLimiter<T> {
    pub fn new(d: Duration) -> Self {
        Self {
            list: (Mutex::new(Default::default())),
            duration: d,
        }
    }
    pub fn is_free(&self, key: &T) -> bool {
        let list = self.list.lock().unwrap();
        let free_at = list.get(key).copied();

        match free_at {
            Some(free_at) => free_at <= Instant::now(),
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
        std::thread::sleep(DURATION - Duration::from_millis(5));
        assert!(!limiter.is_free(&1));
    }

    #[rstest]
    fn it_should_work_with_different_keys(limiter: RateLimiter<u32>) {
        limiter.mark_as_limited(1);
        assert!(limiter.is_free(&2));
    }
}
