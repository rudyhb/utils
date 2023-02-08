use std::time::Duration;

use crate::a_star::AStarNode;

pub struct AStarOptions<TNode: AStarNode> {
    pub(crate) custom_end_condition: Option<Box<dyn Fn(&TNode, &TNode) -> bool>>,
    pub(crate) log_interval: Duration,
    pub(crate) suppress_logs: bool,
    pub(crate) iteration_limit: Option<usize>,
}

impl<TNode: AStarNode> AStarOptions<TNode> {
    pub fn with_ending_condition(
        mut self,
        ending_condition: Box<dyn Fn(&TNode, &TNode) -> bool>,
    ) -> Self {
        self.custom_end_condition = Some(ending_condition);
        self
    }
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

impl<TNode: AStarNode> Default for AStarOptions<TNode> {
    fn default() -> Self {
        Self {
            custom_end_condition: None,
            log_interval: Duration::from_secs(5),
            suppress_logs: false,
            iteration_limit: None,
        }
    }
}
