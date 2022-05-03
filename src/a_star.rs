use anyhow::Result;
use log::*;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::time::Duration;
use thiserror::Error;
use crate::timeout::Timeout;

type TNumber = i32;

pub trait AStarNode: Hash + Eq + PartialEq + Ord + PartialOrd + Send + Sync + Debug {}

trait GetHash: Hash {
    fn get_hash(&self) -> u64;
}

impl<T: Hash> GetHash for T {
    fn get_hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

pub struct AStarOptions<TNode: AStarNode> {
    custom_end_condition: Option<Box<dyn Fn(&TNode, &TNode) -> bool>>,
    log_interval: Duration,
    suppress_logs: bool,
}

impl<TNode: AStarNode> AStarOptions<TNode> {
    pub fn with_ending_condition(
        mut self,
        ending_condition: Box<dyn Fn(&TNode, &TNode) -> bool>,
    ) -> Self {
        self.custom_end_condition = Some(ending_condition);
        self
    }
    pub fn with_log_interval(
        mut self,
        log_interval: Duration,
    ) -> Self {
        self.log_interval = log_interval;
        self
    }
    pub fn with_no_logs(mut self) -> Self {
        self.suppress_logs = true;
        self
    }
}

impl<TNode: AStarNode> Default for AStarOptions<TNode> {
    fn default() -> Self {
        Self {
            custom_end_condition: None,
            log_interval: Duration::from_secs(5),
            suppress_logs: false,
        }
    }
}

pub struct Successor<TNode: AStarNode> {
    node: TNode,
    cost_to_move_here: TNumber,
}

impl<TNode: AStarNode> Successor<TNode> {
    pub fn new(node: TNode, cost_to_move_here: TNumber) -> Self {
        Self {
            node,
            cost_to_move_here,
        }
    }
}

pub struct CurrentNodeDetails<'a, TNode: AStarNode> {
    pub current_node: &'a TNode,
    pub target_node: &'a TNode,
    pub cost_to_move_to_current: TNumber,
}

#[derive(Debug, Error)]
pub enum AStarError {
    #[error("No solution found")]
    NoSolutionFound,
    #[error("An unexpected error occurred")]
    UnexpectedError,
}

struct NodeList<TNode: AStarNode> {
    nodes: HashMap<u64, NodeDetails<TNode>>,
}

impl<TNode: AStarNode> NodeList<TNode> {
    pub(crate) fn new(start: TNode) -> Self {
        let mut result = Self {
            nodes: Default::default(),
        };
        let hash = start.get_hash();
        result.nodes.insert(hash, NodeDetails::new(start, 0, 0));
        result
    }
    pub(crate) fn try_insert_successor(&mut self, details: NodeDetails<TNode>) {
        let hash = details.node.get_hash();
        if let Some(existing) = self.nodes.get(&hash) {
            if existing.f() <= details.f() {
                return;
            }
        }
        self.nodes.insert(hash, details);
    }
    pub(crate) fn get_next(&mut self) -> Result<(&NodeDetails<TNode>, usize)> {
        let index = self
            .nodes
            .values()
            .filter(|node| node.is_open)
            .min_by(sorting_function)
            .map(|details| details.node.get_hash())
            .ok_or(AStarError::NoSolutionFound)?;
        self.nodes.get_mut(&index).unwrap().is_open = false;
        let result = self.nodes.get(&index).ok_or(AStarError::UnexpectedError)?;
        Ok((result, self.nodes.values().filter(|n| n.is_open).count()))
    }
}

pub fn a_star_search<
    TNode: AStarNode,
    TFunc: Fn(&TNode) -> Vec<Successor<TNode>> + Sync + Send,
    TFunc2: Fn(CurrentNodeDetails<TNode>) -> TNumber + Send + Sync,
>(
    start: TNode,
    end: &TNode,
    get_successors: TFunc,
    distance_function: TFunc2,
    options: Option<&AStarOptions<TNode>>,
) -> Result<Vec<TNode>> {
    let default_options = AStarOptions::default();
    let options = options.unwrap_or(&default_options);
    let mut node_list = NodeList::new(start);
    let end_condition = &options.custom_end_condition;
    let mut timeout = Timeout::start(options.log_interval);

    for i in 1.. {
        let (parent, remaining_list_len) = node_list.get_next()?;
        if !options.suppress_logs {
            print_debug(
                parent,
                remaining_list_len,
                if timeout.is_done() {
                    timeout.restart();
                    true
                } else { false },
            );
        }

        let successors: Vec<NodeDetails<TNode>> = {
            let successors = get_successors(&parent.node);
            let mut results: Vec<NodeDetails<TNode>> = Vec::with_capacity(successors.len());
            for Successor {
                node: successor,
                cost_to_move_here: distance,
            } in successors
            {
                let to_current = parent.g + distance;

                if {
                    if let Some(condition) = end_condition.as_ref() {
                        condition(&successor, end)
                    } else {
                        successor == *end
                    }
                } {
                    let end_details = NodeDetails::new_with(successor, to_current, 0, parent);
                    debug!("a_star took {} steps", i);
                    return Ok(make_results(end_details, node_list));
                }

                let to_end = distance_function(CurrentNodeDetails {
                    current_node: &successor,
                    target_node: &end,
                    cost_to_move_to_current: to_current,
                });
                let details = NodeDetails::new_with(successor, to_current, to_end, parent);
                results.push(details);
            }

            results
        };

        for details in successors {
            node_list.try_insert_successor(details);
        }
    }

    Err(AStarError::NoSolutionFound.into())
}

fn print_debug<TNode: AStarNode>(
    q_details: &NodeDetails<TNode>,
    list_len: usize,
    debug_level: bool,
) {
    if debug_level {
        debug!("got {:?}, list_len={}", q_details, list_len);
    } else {
        trace!("got {:?}, list_len={}", q_details, list_len);
    }
}

fn sorting_function<TNode: AStarNode>(
    a: &&NodeDetails<TNode>,
    b: &&NodeDetails<TNode>,
) -> Ordering {
    let c = a.f().cmp(&b.f());
    if c == Ordering::Equal {
        a.node.cmp(&b.node)
    } else {
        c
    }
}

fn make_results<TNode: AStarNode>(
    end: NodeDetails<TNode>,
    mut node_list: NodeList<TNode>,
) -> Vec<TNode> {
    let mut results = vec![end.node];
    let mut parent = end.parent;
    while let Some(parent_hash) = parent {
        let node = node_list.nodes.remove(&parent_hash).unwrap();
        results.push(node.node);
        parent = node.parent;
    }
    results.reverse();
    results
}

struct NodeDetails<TNode: AStarNode> {
    node: TNode,
    is_open: bool,
    g: TNumber,
    h: TNumber,
    parent: Option<u64>,
}

impl<TNode: AStarNode> Debug for NodeDetails<TNode> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut val = format!("{:?}", self.node);
        if val.len() > 130 {
            val = format!("{}..{}", &val[..65], &val[val.len() - 65..])
        }
        write!(f, "q {} with g={}, h={}", val, self.g, self.h)
    }
}

impl<TNode: AStarNode> NodeDetails<TNode> {
    pub(crate) fn new(node: TNode, g: TNumber, h: TNumber) -> Self {
        Self {
            node,
            g,
            h,
            parent: None,
            is_open: true,
        }
    }
    pub(crate) fn new_with(
        node: TNode,
        g: TNumber,
        h: TNumber,
        parent: &NodeDetails<TNode>,
    ) -> Self {
        Self {
            node,
            g,
            h,
            parent: Some(parent.node.get_hash()),
            is_open: true,
        }
    }
    #[inline(always)]
    pub(crate) fn f(&self) -> TNumber {
        self.g + self.h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
    struct TestNode(i32);

    impl AStarNode for TestNode {}

    fn distance_function(node_details: CurrentNodeDetails<TestNode>) -> i32 {
        let CurrentNodeDetails {
            current_node: left,
            target_node: right,
            cost_to_move_to_current: _to_current,
        } = node_details;
        (left.0 - right.0).abs()
    }

    fn get_successors(node: &TestNode) -> Vec<Successor<TestNode>> {
        vec![
            Successor::new(TestNode(node.0 - 1), 1),
            Successor::new(TestNode(node.0 + 1), 1),
        ]
    }

    #[test]
    fn should_find_seven() {
        let start = TestNode(1);
        let target = TestNode(7);

        let solution =
            a_star_search(start, &target, get_successors, distance_function, None).unwrap();

        assert_eq!(
            solution,
            vec![
                TestNode(1),
                TestNode(2),
                TestNode(3),
                TestNode(4),
                TestNode(5),
                TestNode(6),
                TestNode(7),
            ]
        )
    }

    #[test]
    fn should_find_eight() {
        let start = TestNode(1);
        let target = TestNode(7);

        let options = AStarOptions::default().with_ending_condition(Box::new(
            |current: &TestNode, _target: &TestNode| current.0 == 8,
        ));
        let options = Some(&options);

        let solution =
            a_star_search(start, &target, get_successors, distance_function, options).unwrap();

        assert_eq!(
            solution,
            vec![
                TestNode(1),
                TestNode(2),
                TestNode(3),
                TestNode(4),
                TestNode(5),
                TestNode(6),
                TestNode(7),
                TestNode(8),
            ]
        )
    }
}
