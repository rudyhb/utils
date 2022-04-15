use std::time::{Duration, Instant};

pub struct Timer<T>
    where T: Fn(Duration) -> ()
{
    start: Instant,
    on_finish: T,
}

impl<T> Timer<T>
    where T: Fn(Duration) -> ()
{
    pub fn start(on_finish: T) -> Self
        where T: Fn(Duration) -> ()
    {
        Self {
            start: Instant::now(),
            on_finish,
        }
    }
}

impl<T> Drop for Timer<T> where T: Fn(Duration) -> ()
{
    fn drop(&mut self) {
        (self.on_finish)(self.start.elapsed());
    }
}