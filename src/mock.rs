use std::{fmt::Debug, sync::Mutex};

pub struct Mock<OUTPUT: Send + Sync, INPUT: Send + Sync = ()>
where
    INPUT: Clone,
{
    executor: Box<dyn Fn(INPUT) -> OUTPUT + Sync + Send>,
    calls: Mutex<Vec<(std::time::Instant, INPUT)>>,
}
impl<OUTPUT: Send + Sync, INPUT: Clone + Send + Sync> Default for Mock<OUTPUT, INPUT> {
    fn default() -> Self {
        Self::new()
    }
}
impl<OUTPUT: Sync + Send, INPUT: Clone + Send + Sync> Mock<OUTPUT, INPUT> {
    pub fn new() -> Self {
        Self {
            executor: Box::new(|_| panic!("Mock executor not set")),
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn default_output(self) -> Mock<OUTPUT, INPUT>
    where
        OUTPUT: Default,
    {
        Self {
            executor: Box::new(move |_| OUTPUT::default()),
            calls: self.calls,
        }
    }

    pub fn returning(self, output: OUTPUT) -> Mock<OUTPUT, INPUT>
    where
        OUTPUT: Clone + 'static,
    {
        Self {
            executor: Box::new(move |_| output.clone()),
            calls: self.calls,
        }
    }

    pub fn generator<T: Send + Sync>(self, output: T) -> Mock<OUTPUT, INPUT>
    where
        T: Fn() -> OUTPUT + 'static + Sync + Sync,
    {
        Self {
            executor: Box::new(move |_| output()),
            calls: self.calls,
        }
    }

    pub fn fake<T: Send + Sync>(self, fake: T) -> Mock<OUTPUT, INPUT>
    where
        T: Fn(INPUT) -> OUTPUT + 'static + Sync + Sync,
    {
        Self {
            executor: Box::new(fake),
            calls: self.calls,
        }
    }

    pub fn new_returning_default() -> Mock<OUTPUT, INPUT>
    where
        OUTPUT: Default,
    {
        Self::default_output(Self::new())
    }
    pub fn new_returning(output: OUTPUT) -> Mock<OUTPUT, INPUT>
    where
        OUTPUT: Clone + 'static,
    {
        Self::new().returning(output)
    }

    pub fn new_returning_not_clone<T: Send + Sync>(output: T) -> Mock<OUTPUT, INPUT>
    where
        T: Fn() -> OUTPUT + 'static + Sync,
    {
        Self::new().generator(output)
    }

    pub fn new_fake<T: Send + Sync>(fake: T) -> Mock<OUTPUT, INPUT>
    where
        T: Fn(INPUT) -> OUTPUT + 'static + Sync,
    {
        Self::new().fake(fake)
    }

    pub fn call(&self, input: INPUT) -> OUTPUT {
        let result = (self.executor)(input.clone());
        {
            let mut calls = self.calls.lock().unwrap();
            calls.push((std::time::Instant::now(), input));
        }
        result
    }
}

impl<OUTPUT: Send + Sync, INPUT: Clone + Send + Sync> Mock<OUTPUT, INPUT> {
    pub fn get_calls(&self) -> Vec<INPUT> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|(_, args)| args.clone())
            .collect()
    }
    pub fn get_calls_with_time(&self) -> Vec<(std::time::Instant, INPUT)> {
        self.calls.lock().unwrap().clone()
    }
    pub fn get_nth_call(&self, index: usize) -> Option<INPUT> {
        let calls = self.calls.lock().unwrap();
        calls.get(index).map(|e| e.1.clone())
    }
    pub fn get_nth_call_with_time(&self, index: usize) -> Option<(std::time::Instant, INPUT)> {
        let calls = self.calls.lock().unwrap();
        calls.get(index).cloned()
    }
}

impl<OUTPUT: Send + Sync, INPUT: Clone + Send + Sync> Mock<OUTPUT, INPUT> {
    pub fn assert_called_times(&self, times: usize) {
        let calls = self.calls.lock().unwrap();
        assert_eq!(calls.len(), times);
    }
    pub fn assert_nth_call(&self, n: usize, _input: INPUT)
    where
        INPUT: PartialEq + Debug,
    {
        self.get_nth_call(n);
    }
    pub fn assert_first_call(&self, input: INPUT)
    where
        INPUT: PartialEq + Debug,
    {
        self.assert_nth_call(0, input)
    }
    pub fn assert_called(&self) {
        let length = self.calls.lock().unwrap().len();
        assert!(length > 0, "Expected call but no calls made");
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn it_must_return_default_output() {
        let mock: Mock<u32> = Mock::new().default_output();
        let result = mock.call(());
        assert_eq!(result, 0);
    }

    #[test]
    fn it_must_return_specified_value() {
        let mock: Mock<i32> = Mock::new().returning(42);
        let result = mock.call(());
        assert_eq!(result, 42);
    }

    mod generator {
        use super::*;
        use std::sync::Arc;

        #[test]
        fn it_must_call_exactly_as_much_as_needed_generator() {
            let count = Arc::new(Mutex::new(0));
            let count_clone = count.clone();
            let mock: Mock<i32> = Mock::new().generator(move || {
                let mut count = count.lock().unwrap();
                *count += 1;
                42
            });
            mock.call(());
            mock.call(());
            mock.call(());

            assert!(count_clone.lock().unwrap().eq(&3));
        }

        #[test]
        fn it_must_return_generated_value() {
            let mock: Mock<i32> = Mock::new().generator(|| 42);
            let result = mock.call(());
            assert_eq!(result, 42);
        }
    }
    mod fake {
        use super::*;
        #[test]
        fn it_must_call_fake_with_needed_args() {
            let mock: Mock<i32, i32> = Mock::new().fake(|a| {
                assert_eq!(a, 12);
                42
            });
            let result = mock.call(12);

            assert_eq!(result, 42);
        }

        #[test]
        fn it_must_return_fake_value() {
            let mock: Mock<i32, i32> = Mock::new().fake(|_| 42);
            let result = mock.call(12);

            assert_eq!(result, 42);
        }
    }

    mod getters {
        use super::*;
        #[test]
        fn it_must_return_calls() {
            let mock: Mock<i32, i32> = Mock::new().fake(|_| 42);
            mock.call(12);
            mock.call(13);
            mock.call(14);

            let calls = mock.get_calls();
            assert_eq!(calls, vec![12, 13, 14]);
        }
    }
    // TODO: add more tests
}
