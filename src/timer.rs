use std::time::{Duration, Instant};

pub struct Timer<T>
where
    T: FnMut(Duration) -> (),
{
    start: Instant,
    on_finish: T,
}

impl<T> Timer<T>
where
    T: FnMut(Duration) -> (),
{
    pub fn start(on_finish: T) -> Self
    where
        T: FnMut(Duration) -> (),
    {
        Self {
            start: Instant::now(),
            on_finish,
        }
    }
}

impl<T> Drop for Timer<T>
where
    T: FnMut(Duration) -> (),
{
    fn drop(&mut self) {
        (self.on_finish)(self.start.elapsed());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_update_value() {
        let mut value = 0i32;
        {
            let _timer = Timer::start(|_| {
                value = 1;
            });
        }
        assert_eq!(value, 1);
    }
}
