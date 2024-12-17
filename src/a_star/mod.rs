use log::*;

use crate::common::Numeric;
use crate::timeout::Timeout;
pub use models::{ComputationResult, CurrentNodeDetails, Error, Node, Result, Successor};
use models::{NodeDetails, NodeList};
pub use options::Options;

mod helpers;
mod implementations;
mod models;
mod options;

pub fn a_star_search<
    TNode: Node,
    TFunc: FnMut(&TNode) -> Vec<Successor<TNode, TNumber>> + Sync + Send,
    TFunc2: FnMut(CurrentNodeDetails<TNode, TNumber>) -> TNumber + Send + Sync,
    TNumber: Numeric,
>(
    start: TNode,
    end: &TNode,
    mut get_successors: TFunc,
    mut distance_function: TFunc2,
    options: Option<&Options<TNode>>,
) -> Result<ComputationResult<TNode, TNumber>> {
    let default_options = Options::default();
    let options = options.unwrap_or(&default_options);
    let mut node_list = NodeList::new(start);
    let end_condition = &options.custom_end_condition;
    let mut timeout = Timeout::start(options.log_interval);

    for i in 1usize..options.iteration_limit.unwrap_or(usize::MAX) {
        let (parent, remaining_list_len) = node_list.get_next()?;
        if !options.suppress_logs {
            if timeout.is_done() {
                debug!("got {:?}, list_len={}", parent, remaining_list_len);
                timeout.restart();
            } else {
                trace!("got {:?}, list_len={}", parent, remaining_list_len);
            }
        }

        let successors: Vec<NodeDetails<TNode, TNumber>> = {
            let successors = get_successors(&parent.node);
            let mut results: Vec<NodeDetails<TNode, TNumber>> =
                Vec::with_capacity(successors.len());
            for Successor {
                node: successor,
                cost_to_move_here: distance,
            } in successors
            {
                let to_current = parent.current_accrued_cost + distance;

                let is_at_end = if let Some(condition) = end_condition.as_ref() {
                    condition(&successor, end)
                } else {
                    successor == *end
                };
                if is_at_end {
                    let end_details =
                        NodeDetails::new_with_parent(successor, to_current, TNumber::default(), parent);
                    if !options.suppress_logs {
                        debug!("a_star took {} steps", i);
                    }
                    return Ok(make_results(end_details, node_list));
                }

                let to_end = distance_function(CurrentNodeDetails {
                    current_node: &successor,
                    target_node: end,
                    cost_to_move_to_current: to_current,
                });
                let details = NodeDetails::new_with_parent(successor, to_current, to_end, parent);
                results.push(details);
            }

            results
        };

        for details in successors {
            node_list.try_insert_successor(details);
        }
    }

    Err(Error::IterLimitExceeded)
}

pub fn a_star_search_all_with_max_score<
    TNode: Node + Clone,
    TFunc: FnMut(&TNode) -> Vec<Successor<TNode, TNumber>> + Sync + Send,
    TFunc2: FnMut(CurrentNodeDetails<TNode, TNumber>) -> TNumber + Send + Sync,
    TNumber: Numeric,
>(
    max_score: TNumber,
    start: TNode,
    end: &TNode,
    mut get_successors: TFunc,
    mut distance_function: TFunc2,
    options: Option<&Options<TNode>>,
) -> Result<Vec<ComputationResult<TNode, TNumber>>> {
    let default_options = Options::default();
    let options = options.unwrap_or(&default_options);
    let mut node_list = NodeList::new(start);
    let end_condition = &options.custom_end_condition;
    let mut timeout = Timeout::start(options.log_interval);

    let mut scoring_results: Vec<NodeDetails<TNode, TNumber>> = vec![];

    for _ in 1usize..options.iteration_limit.unwrap_or(usize::MAX) {
        let parent = if let Ok((parent, remaining_list_size)) = node_list.get_next() {
            if !options.suppress_logs {
                if timeout.is_done() {
                    debug!("got {:?}, list_len={}", parent, remaining_list_size);
                    timeout.restart();
                } else {
                    trace!("got {:?}, list_len={}", parent, remaining_list_size);
                }
            }
            parent
        } else {
            return if scoring_results.is_empty() {
                Err(Error::NoSolutionFound)
            } else {
                Ok(make_results_multiple(scoring_results, node_list))
            };
        };

        let successors: Vec<NodeDetails<TNode, TNumber>> = {
            let successors = get_successors(&parent.node);
            let mut next_parents: Vec<NodeDetails<TNode, TNumber>> =
                Vec::with_capacity(successors.len());
            for Successor {
                node: successor,
                cost_to_move_here: distance,
            } in successors
            {
                let to_current = parent.current_accrued_cost + distance;
                if to_current > max_score {
                    continue;
                }
                
                let is_at_end = if let Some(condition) = end_condition.as_ref() {
                    condition(&successor, end)
                } else {
                    successor == *end
                };
                if is_at_end {
                    let end_details =
                        NodeDetails::new_with_parent(successor, to_current, TNumber::default(), parent);
                    
                    scoring_results.push(end_details);
                    continue;
                }

                let to_end = distance_function(CurrentNodeDetails {
                    current_node: &successor,
                    target_node: end,
                    cost_to_move_to_current: to_current,
                });
                let details = NodeDetails::new_with_parent(successor, to_current, to_end, parent);
                next_parents.push(details);
            }

            next_parents
        };

        for details in successors {
            node_list.try_insert_successor(details);
        }
    }

    Err(Error::IterLimitExceeded)
}

fn make_results<TNode: Node, TNumber: Numeric>(
    end: NodeDetails<TNode, TNumber>,
    mut node_list: NodeList<TNode, TNumber>,
) -> ComputationResult<TNode, TNumber> {
    let shortest_path_cost = end.current_accrued_cost;
    let mut results = vec![end.node];
    let mut parent = end.parent;
    while let Some(parent_hash) = parent {
        let node = node_list.node_history.remove(&parent_hash).unwrap();
        results.push(node.node);
        parent = node.parent;
    }
    results.reverse();
    ComputationResult {
        shortest_path: results,
        shortest_path_cost,
    }
}

fn make_results_multiple<TNode: Node + Clone, TNumber: Numeric>(
    end: Vec<NodeDetails<TNode, TNumber>>,
    node_list: NodeList<TNode, TNumber>,
) -> Vec<ComputationResult<TNode, TNumber>> {
    let mut all_results = vec![];
    for end in end {
        let shortest_path_cost = end.current_accrued_cost;
        let mut results = vec![end.node];
        let mut parent = end.parent;
        while let Some(parent_hash) = parent {
            let node = node_list.node_history.get(&parent_hash).unwrap();
            results.push(node.node.clone());
            parent = node.parent;
        }
        results.reverse();
        all_results.push(ComputationResult {
            shortest_path: results,
            shortest_path_cost,
        });
    }
    all_results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
    struct TestNode(i32);

    impl Node for TestNode {}

    fn distance_function(node_details: CurrentNodeDetails<TestNode, i32>) -> i32 {
        let CurrentNodeDetails {
            current_node: left,
            target_node: right,
            cost_to_move_to_current: _to_current,
        } = node_details;
        (left.0 - right.0).abs()
    }

    fn get_successors(node: &TestNode) -> Vec<Successor<TestNode, i32>> {
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

        let options = Options::default().with_ending_condition(Box::new(
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
