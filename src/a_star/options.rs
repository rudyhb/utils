use std::fmt::{Debug, Formatter};
use std::time::Duration;

pub struct Options {
    pub(crate) log_interval: Duration,
    pub(crate) suppress_logs: bool,
    pub(crate) iteration_limit: Option<usize>,
}

impl Debug for Options {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "log={} interval={:?} iter_limit={:?}",
            !self.suppress_logs,
            if self.suppress_logs {
                None
            } else {
                Some(self.log_interval)
            },
            self.iteration_limit,
        )
    }
}

impl Options {
    pub fn with_log_interval(mut self, log_interval: Duration) -> Self {
        self.log_interval = log_interval;
        self
    }
    pub fn with_no_logs(mut self) -> Self {
        self.suppress_logs = true;
        self
    }
    pub fn with_iteration_limit(mut self, limit: usize) -> Self {
        self.iteration_limit = Some(limit);
        self
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            log_interval: Duration::from_secs(5),
            suppress_logs: false,
            iteration_limit: None,
        }
    }
}
