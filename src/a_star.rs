use std::cmp::Ordering;
use std::collections::{HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

type TNumber = i32;

pub trait AStarNode: Hash + Eq + PartialEq + Clone + Ord + PartialOrd + Send + std::marker::Sync + Debug {}

pub struct AStarOptions {
    print_debug: bool,
    print_current_val: bool,
    print_every: Option<usize>,
    run_in_parallel: bool,
}

impl AStarOptions {
    pub fn print_stats() -> Self {
        let mut s = Self::default();
        s.print_debug = true;
        s
    }
    pub fn print_stats_and_values() -> Self {
        let mut s = Self::print_stats();
        s.print_current_val = true;
        s
    }
    pub fn print_stats_and_values_every(x: usize) -> Self {
        let mut s = Self::print_stats_and_values();
        s.print_every = Some(x);
        s
    }
    pub fn run_in_parallel(mut self) -> Self {
        self.run_in_parallel = true;
        self
    }
}

impl Default for AStarOptions {
    fn default() -> Self {
        Self {
            print_debug: false,
            print_current_val: false,
            print_every: None,
            run_in_parallel: false,
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

pub type Result<TNode> = std::result::Result<Vec<TNode>, AStarError>;

#[derive(Debug)]
pub enum AStarError {
    NoSolutionFound,
    MutexError(&'static str)
}

pub fn a_star_search<TNode: AStarNode, TFunc: Fn(&TNode) -> Vec<Successor<TNode>> + std::marker::Sync + std::marker::Send, TFunc2: Fn(CurrentNodeDetails<TNode>) -> TNumber + Send + std::marker::Sync>(start: TNode, end: &TNode, get_successors: TFunc, distance_function: TFunc2, options: Option<&AStarOptions>) -> Result<TNode> {
    let default_options = AStarOptions::default();
    let options = options.unwrap_or(&default_options);
    let mut closed_list: HashMap<TNode, NodeDetails<TNode>> = Default::default();
    let mut open_list: HashMap<TNode, NodeDetails<TNode>> = Default::default();
    open_list.insert(start, NodeDetails::new(0, 0));

    let mut i = 0usize;
    while !open_list.is_empty() {
        i += 1;

        let (q_nodes, list_len) = {
            let q_nodes = if options.run_in_parallel {
                let threads = crate::num_cpus::get();

                let mut q_nodes: Vec<_> = open_list.iter().collect();
                q_nodes.sort_by(sorting_function);
                let q_nodes = q_nodes.into_iter().take(threads)
                    .map(|(key, _)| key).cloned().collect::<Vec<_>>();
                q_nodes.into_iter()
                    .map(|q_node| {
                        let q_details = open_list.remove(&q_node).unwrap();
                        (q_node, q_details)
                    })
                    .collect::<Vec<_>>()
            } else {
                let q_node = open_list.iter().min_by(sorting_function).map(|(key, _)| key).cloned().expect("no minimum");
                let q_details = open_list.remove(&q_node).unwrap();
                vec![(q_node, q_details)]
            };

            (q_nodes, open_list.len())
        };

        if options.print_debug {
            let print = if let Some(every) = options.print_every {
                i % every == 0
            } else {
                true
            };
            if print {
                if let Some((q_node, q_details)) = q_nodes.iter().next() {
                    if options.print_current_val {
                        let mut val = format!("{:?}", q_node);
                        if val.len() > 130 {
                            val = format!("{}..{}", &val[..65], &val[val.len() - 65..])
                        }
                        println!("got q {} with g={}, h={}, list_len={}", val, q_details.g, q_details.h, list_len);
                    } else {
                        println!("got q with g={}, h={}, list_len={}", q_details.g, q_details.h, list_len);
                    }
                }
            }
        }

        let successors = {
            if q_nodes.len() == 1 {
                let (q_node, q_details) = q_nodes.iter().next().unwrap();
                let (successors, done) = run_get_successors(q_node, q_details, end, &get_successors, &distance_function);
                if let Some((node, details)) = done {
                    return Ok(make_results(node, details));
                }
                successors
            } else {
                let successors: Vec<(TNode, NodeDetails<TNode>)> = Default::default();
                let final_results: Vec<(TNode, NodeDetails<TNode>)> = Default::default();
                let successors: Arc<Mutex<_>> = Arc::new(Mutex::new(successors));
                let final_results: Arc<Mutex<_>> = Arc::new(Mutex::new(final_results));

                rayon::scope(|s| {
                    for (q_node, q_details) in q_nodes.iter() {
                        let result_successors = Arc::clone(&successors);
                        let final_results = Arc::clone(&final_results);
                        let get_successors = &get_successors;
                        let distance_function = &distance_function;
                        s.spawn(move |_| {
                            let (successors, done) = run_get_successors(q_node, q_details, end, get_successors, distance_function);
                            if let Some((node, details)) = done {
                                let mut final_results = final_results.lock().unwrap();
                                final_results.push((node, details));
                                return;
                            }
                            let mut result_successors = result_successors.lock().unwrap();
                            result_successors.extend(successors);
                        })
                    }
                });

                let lock = Arc::try_unwrap(final_results).or(Err(AStarError::MutexError("lock still has multiple owners")))?;
                let final_results = lock.into_inner().or(Err(AStarError::MutexError("mutex cannot be locked")))?;
                if !final_results.is_empty() {
                    let (node, details) = final_results.into_iter().min_by(|(_, a), (_, b)| a.g.cmp(&b.g))
                        .unwrap();
                    return Ok(make_results(node, details));
                }

                let lock = Arc::try_unwrap(successors).or(Err(AStarError::MutexError("lock still has multiple owners")))?;
                let successors = lock.into_inner().or(Err(AStarError::MutexError("mutex cannot be locked")))?;

                successors
            }
        };

        for (successor, details) in successors {
            if let Some(existing) = open_list.get(&successor) {
                if existing.f() < details.f() {
                    continue;
                }
            }
            if let Some(existing) = closed_list.get(&successor) {
                if existing.f() < details.f() {
                    continue;
                }
            }
            open_list.insert(successor, details);
        }

        closed_list.extend(q_nodes);
    }

    Err(AStarError::NoSolutionFound)
}

fn run_get_successors<TNode: AStarNode, TFunc: Fn(&TNode) -> Vec<Successor<TNode>>, TFunc2: Fn(CurrentNodeDetails<TNode>) -> TNumber + Send + std::marker::Sync>(q_node: &TNode, q_details: &NodeDetails<TNode>, end: &TNode, get_successors: &TFunc, distance_function: &TFunc2) -> (Vec<(TNode, NodeDetails<TNode>)>, Option<(TNode, NodeDetails<TNode>)>) {
    let successors = get_successors(&q_node);
    let mut results: Vec<(TNode, NodeDetails<TNode>)> = Vec::with_capacity(successors.len());
    for Successor {
        node: successor,
        cost_to_move_here: distance
    } in successors {
        let to_current = q_details.g + distance;

        if successor == *end {
            let details = NodeDetails::new_with(to_current, 0, q_node.clone(), q_details.clone());
            return (vec![], Some((successor, details)));
        }

        let to_end = distance_function(CurrentNodeDetails {
            current_node: &successor,
            target_node: &end,
            cost_to_move_to_current: to_current,
        });
        let details = NodeDetails::new_with(to_current, to_end, q_node.clone(), q_details.clone());
        results.push((successor, details));
    }

    (results, None)
}

fn sorting_function<TNode: AStarNode>((a_node, a): &(&TNode, &NodeDetails<TNode>), (b_node, b): &(&TNode, &NodeDetails<TNode>)) -> Ordering {
    let c = a.f().cmp(&b.f());
    if c == Ordering::Equal {
        a_node.cmp(&b_node)
    } else {
        c
    }
}

fn make_results<'a, TNode: AStarNode>(end: TNode, details: NodeDetails<TNode>) -> Vec<TNode> {
    let mut results = vec![end];
    let mut parent = details.parent;
    while let Some(parent_details) = parent {
        let (parent_node, parent_details) = *parent_details;
        results.push(parent_node);
        parent = parent_details.parent;
    }
    results.reverse();
    results
}


#[derive(Clone)]
struct NodeDetails<TNode: AStarNode> {
    g: TNumber,
    h: TNumber,
    parent: Option<Box<(TNode, NodeDetails<TNode>)>>,
}

impl<TNode: AStarNode> NodeDetails<TNode> {
    pub(crate) fn new(g: TNumber, h: TNumber) -> Self {
        Self { g, h, parent: None }
    }
    pub(crate) fn new_with(g: TNumber, h: TNumber, parent: TNode, parent_details: NodeDetails<TNode>) -> Self {
        Self { g, h, parent: Some(Box::new((parent, parent_details))) }
    }
    #[inline]
    pub(crate) fn f(&self) -> TNumber {
        self.g + self.h
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use super::*;

    #[derive(Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Debug)]
    struct TestNode(i32);

    impl AStarNode for TestNode {}

    fn distance_function(node_details: CurrentNodeDetails<TestNode>) -> i32 {
        let CurrentNodeDetails {
            current_node: left,
            target_node: right,
            cost_to_move_to_current: _to_current
        } = node_details;
        (left.0 - right.0).abs()
    }

    fn get_successors(node: &TestNode) -> Vec<Successor<TestNode>> {
        vec![
            Successor::new(TestNode(node.0 - 1), 1),
            Successor::new(TestNode(node.0 + 1), 1),
        ]
    }

    fn distance_function_delay(node_details: CurrentNodeDetails<TestNode>) -> i32 {
        std::thread::sleep(Duration::from_millis(100));
        let CurrentNodeDetails {
            current_node: left,
            target_node: right,
            cost_to_move_to_current: _to_current
        } = node_details;
        (left.0 - right.0).abs()
    }

    fn get_successors_wormhole(node: &TestNode) -> Vec<Successor<TestNode>> {
        if node.0 == -1 {
            return vec![Successor::new(TestNode(7), 1)];
        }
        vec![
            Successor::new(TestNode(node.0 - 1), 1),
            Successor::new(TestNode(node.0 + 1), 1),
        ]
    }

    #[test]
    fn should_find_seven() {
        let start = TestNode(1);
        let target = TestNode(7);

        let solution = a_star_search(start, &target, get_successors, distance_function, None).unwrap();

        assert_eq!(solution, vec![TestNode(1), TestNode(2), TestNode(3), TestNode(4), TestNode(5), TestNode(6), TestNode(7)])
    }

    #[test]
    fn should_find_seven_in_parallel() {
        let start = TestNode(1);
        let target = TestNode(7);

        let options = AStarOptions::print_stats_and_values().run_in_parallel();
        let options = Some(&options);

        let solution = {
            let _timer = crate::timer::Timer::start(|elapsed| {
                println!("elapsed: {} ms", elapsed.as_millis());
                assert!(elapsed.as_millis() < 1000);
            });
            a_star_search(start, &target, get_successors_wormhole, distance_function_delay, options).unwrap()
        };

        assert_eq!(solution, vec![TestNode(1), TestNode(0), TestNode(-1), TestNode(7)])
    }
}