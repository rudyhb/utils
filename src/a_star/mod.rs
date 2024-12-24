use log::*;

use crate::common::Numeric;
use crate::timeout::Timeout;
pub use models::{
    ComputationResult, CurrentNodeDetails, CustomNode, Error, Node, Result, Successor,
};
use models::{NodeDetails, NodeList};
pub use options::Options;

mod helpers;
mod implementations;
mod models;
mod options;

pub fn a_star_search<
    TNode: CustomNode,
    TSuccessorsFunc: FnMut(&TNode) -> Vec<Successor<TNode, TNumber>> + Sync + Send,
    TDistanceFunc: FnMut(CurrentNodeDetails<TNode, TNumber>) -> TNumber + Send + Sync,
    TEndCheckFunc: FnMut(&TNode) -> bool,
    TNumber: Numeric,
>(
    start: TNode,
    mut get_successors: TSuccessorsFunc,
    mut distance_function: TDistanceFunc,
    mut is_at_end_function: TEndCheckFunc,
    options: Option<&Options>,
) -> Result<ComputationResult<TNode, TNumber>> {
    let default_options = Options::default();
    let options = options.unwrap_or(&default_options);
    let mut node_list = NodeList::new(start);
    let mut timeout = Timeout::start(options.log_interval);

    if !options.suppress_logs {
        debug!("[a*] starting a* search with options {:?}", options);
    }

    for i in 1usize..options.iteration_limit.unwrap_or(usize::MAX) {
        let (parent, remaining_list_len) = node_list.get_next()?;
        if !options.suppress_logs {
            trace!(
                "[a*] step={} got {:?}, list_len={}",
                i,
                parent,
                remaining_list_len
            );
            if timeout.is_done() {
                debug!(
                    "[a*] step={} list_len={}, current_accrued_cost={}",
                    i, remaining_list_len, parent.current_accrued_cost
                );
                timeout.restart();
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

                if is_at_end_function(&successor) {
                    let end_details = NodeDetails::new_with_parent(
                        successor,
                        to_current,
                        TNumber::default(),
                        parent,
                    );
                    if !options.suppress_logs {
                        debug!("[a*] took {} steps", i);
                    }
                    return Ok(make_results(end_details, node_list));
                }

                let to_end = distance_function(CurrentNodeDetails {
                    current_node: &successor,
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
    TNode: CustomNode + Clone,
    TSuccessorsFunc: FnMut(&TNode) -> Vec<Successor<TNode, TNumber>> + Sync + Send,
    TDistanceFunc: FnMut(CurrentNodeDetails<TNode, TNumber>) -> TNumber + Send + Sync,
    TEndCheckFunc: FnMut(&TNode) -> bool,
    TNumber: Numeric,
>(
    max_score: TNumber,
    start: TNode,
    mut get_successors: TSuccessorsFunc,
    mut distance_function: TDistanceFunc,
    mut is_at_end_function: TEndCheckFunc,
    options: Option<&Options>,
) -> Result<Vec<ComputationResult<TNode, TNumber>>> {
    let default_options = Options::default();
    let options = options.unwrap_or(&default_options);
    let mut node_list = NodeList::new(start);
    let mut timeout = Timeout::start(options.log_interval);

    if !options.suppress_logs {
        debug!(
            "[a*] starting search all with max score of {} and options {:?}",
            max_score, options
        );
    }

    let mut scoring_results: Vec<NodeDetails<TNode, TNumber>> = vec![];

    for i in 1usize..options.iteration_limit.unwrap_or(usize::MAX) {
        let parent = if let Ok((parent, remaining_list_size)) = node_list.get_next() {
            if !options.suppress_logs {
                trace!(
                    "[a*] step={} got {:?}, list_len={}",
                    i,
                    parent,
                    remaining_list_size
                );
                if timeout.is_done() {
                    debug!(
                        "[a*] step={} list_len={}, results_len={} current_accrued_cost={}/{}",
                        i,
                        remaining_list_size,
                        scoring_results.len(),
                        parent.current_accrued_cost,
                        max_score
                    );
                    timeout.restart();
                }
            }
            parent
        } else {
            return if scoring_results.is_empty() {
                Err(Error::NoSolutionFound)
            } else {
                if !options.suppress_logs {
                    debug!("[a*] took {} steps", i);
                }
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

                if is_at_end_function(&successor) {
                    let end_details = NodeDetails::new_with_parent(
                        successor,
                        to_current,
                        TNumber::default(),
                        parent,
                    );

                    scoring_results.push(end_details);
                    continue;
                }

                let to_end = distance_function(CurrentNodeDetails {
                    current_node: &successor,
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

fn make_results<TNode: CustomNode, TNumber: Numeric>(
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

fn make_results_multiple<TNode: CustomNode + Clone, TNumber: Numeric>(
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

    fn distance_function(
        node_details: CurrentNodeDetails<TestNode, i32>,
        target_node: &TestNode,
    ) -> i32 {
        let CurrentNodeDetails {
            current_node: left,
            cost_to_move_to_current: _to_current,
        } = node_details;
        (left.0 - target_node.0).abs()
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

        let solution = a_star_search(
            start,
            get_successors,
            |current| distance_function(current, &target),
            |current| current == &target,
            None,
        )
        .unwrap();

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

        let options = Options::default();
        let options = Some(&options);

        let solution = a_star_search(
            start,
            get_successors,
            |current| distance_function(current, &target),
            |current| current.0 == 8,
            options,
        )
        .unwrap();

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

    #[test]
    fn should_find_both_paths() {
        let start = TestNode(0);
        let target = TestNode(25);

        let solution = a_star_search_all_with_max_score(
            5,
            start,
            |node| {
                vec![
                    Successor::new(TestNode(node.0 - 1), 1),
                    Successor::new(TestNode(node.0 + 1), 1),
                ]
            },
            |node| (node.current_node.0.pow(2) - target.0).abs(),
            |current| current.0.pow(2) == target.0,
            None,
        )
        .unwrap();

        assert_eq!(solution.len(), 2);

        assert!(solution.iter().any(|solution| {
            solution.shortest_path
                == vec![
                    TestNode(0),
                    TestNode(1),
                    TestNode(2),
                    TestNode(3),
                    TestNode(4),
                    TestNode(5),
                ]
        }));
        assert!(solution.iter().any(|solution| {
            solution.shortest_path
                == vec![
                    TestNode(0),
                    TestNode(-1),
                    TestNode(-2),
                    TestNode(-3),
                    TestNode(-4),
                    TestNode(-5),
                ]
        }));
        assert_eq!(solution[0].shortest_path_cost, 5);
        assert_eq!(solution[1].shortest_path_cost, 5);
    }
    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestNode2(i32);

    impl CustomNode for TestNode2 {
        const NODE_ID_AND_POSITION_HASH_SAME: bool = false;

        fn get_node_id(&self) -> u64 {
            ((i32::MAX / 2) + self.0) as u64
        }

        fn get_position_hash(&self) -> u64 {
            self.0.unsigned_abs() as u64
        }
    }
    #[test]
    fn should_find_both_paths_different_position_hash() {
        let start = TestNode2(0);
        let target = TestNode2(25);

        let solution = a_star_search_all_with_max_score(
            5,
            start,
            |node| {
                vec![
                    Successor::new(TestNode2(node.0 - 1), 1),
                    Successor::new(TestNode2(node.0 + 1), 1),
                ]
            },
            |node| (node.current_node.0.pow(2) - target.0).abs(),
            |current| current.0.pow(2) == target.0,
            None,
        )
        .unwrap();

        assert_eq!(solution.len(), 2);

        assert!(solution.iter().any(|solution| {
            solution.shortest_path
                == vec![
                    TestNode2(0),
                    TestNode2(1),
                    TestNode2(2),
                    TestNode2(3),
                    TestNode2(4),
                    TestNode2(5),
                ]
        }));
        assert!(solution.iter().any(|solution| {
            solution.shortest_path
                == vec![
                    TestNode2(0),
                    TestNode2(-1),
                    TestNode2(-2),
                    TestNode2(-3),
                    TestNode2(-4),
                    TestNode2(-5),
                ]
        }));
        assert_eq!(solution[0].shortest_path_cost, 5);
        assert_eq!(solution[1].shortest_path_cost, 5);
    }
}
