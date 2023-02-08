use std::fmt::Debug;
use std::hash::Hash;

use anyhow::Result;
use log::*;

pub use models::{AStarError, AStarResult, CurrentNodeDetails, Successor};
use models::{NodeDetails, NodeList};
pub use options::AStarOptions;

use crate::timeout::Timeout;

mod helpers;
mod models;
mod options;

type TNumber = i32;

pub trait AStarNode: Hash + Eq + PartialEq + Ord + PartialOrd + Send + Sync + Debug {}

pub fn a_star_search<
    TNode: AStarNode,
    TFunc: FnMut(&TNode) -> Vec<Successor<TNode>> + Sync + Send,
    TFunc2: FnMut(CurrentNodeDetails<TNode>) -> TNumber + Send + Sync,
>(
    start: TNode,
    end: &TNode,
    mut get_successors: TFunc,
    mut distance_function: TFunc2,
    options: Option<&AStarOptions<TNode>>,
) -> Result<AStarResult<TNode>> {
    let default_options = AStarOptions::default();
    let options = options.unwrap_or(&default_options);
    let mut node_list = NodeList::new(start);
    let end_condition = &options.custom_end_condition;
    let mut timeout = Timeout::start(options.log_interval);

    for i in 1usize..options.iteration_limit.unwrap_or(usize::MAX) {
        let (parent, remaining_list_len) = node_list.get_next()?;
        if !options.suppress_logs {
            print_debug(
                parent,
                remaining_list_len,
                if timeout.is_done() {
                    timeout.restart();
                    true
                } else {
                    false
                },
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
                    if !options.suppress_logs {
                        debug!("a_star took {} steps", i);
                    }
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

    Err(AStarError::IterLimitExceeded.into())
}

#[inline]
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

fn make_results<TNode: AStarNode>(
    end: NodeDetails<TNode>,
    mut node_list: NodeList<TNode>,
) -> AStarResult<TNode> {
    let shortest_path_cost = end.g;
    let mut results = vec![end.node];
    let mut parent = end.parent;
    while let Some(parent_hash) = parent {
        let node = node_list.nodes.remove(&parent_hash).unwrap();
        results.push(node.node);
        parent = node.parent;
    }
    results.reverse();
    AStarResult {
        shortest_path: results,
        shortest_path_cost,
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
            solution.shortest_path,
            vec![
                TestNode(1),
                TestNode(2),
                TestNode(3),
                TestNode(4),
                TestNode(5),
                TestNode(6),
                TestNode(7),
            ]
        );
        assert_eq!(solution.shortest_path_cost, 6);
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
            solution.shortest_path,
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
        );
        assert_eq!(solution.shortest_path_cost, 7);
    }
}
