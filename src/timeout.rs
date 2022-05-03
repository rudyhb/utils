use std::time::{Duration, Instant};

pub struct Timeout
{
    start: Instant,
    duration: Duration,
}

impl Timeout
{
    pub fn start(duration: Duration) -> Self
    {
        Self {
            start: Instant::now(),
            duration,
        }
    }
    pub fn is_done(&self) -> bool {
        self.start.elapsed() >= self.duration
    }
    pub fn restart(&mut self) {
        self.start = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_finish_on_time() {
        let timeout = Timeout::start(Duration::from_millis(100));
        std::thread::sleep(Duration::from_millis(50));
        assert!(!timeout.is_done());
        std::thread::sleep(Duration::from_millis(60));
        assert!(timeout.is_done());
    }

    #[test]
    fn should_restart() {
        let mut timeout = Timeout::start(Duration::from_millis(50));
        std::thread::sleep(Duration::from_millis(60));
        assert!(timeout.is_done());
        timeout.restart();
        assert!(!timeout.is_done());
        std::thread::sleep(Duration::from_millis(60));
        assert!(timeout.is_done());
    }
}
