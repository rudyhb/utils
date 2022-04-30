use std::cmp::Ordering;
use std::collections::{HashMap};
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use rayon::prelude::*;
use log::*;

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

pub struct AStarOptions {
    run_in_parallel: bool,
}

impl AStarOptions {
    pub fn run_in_parallel(mut self) -> Self {
        self.run_in_parallel = true;
        self
    }
}

impl Default for AStarOptions {
    fn default() -> Self {
        Self {
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
    MutexError(&'static str),
}


struct NodeList<TNode: AStarNode> {
    nodes: HashMap<u64, NodeDetails<TNode>>,
}

impl<TNode: AStarNode> NodeList<TNode> {
    pub(crate) fn new(start: TNode) -> Self {
        let mut result = Self {
            nodes: Default::default()
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
    pub(crate) fn get_next(&mut self, count: usize) -> Option<(Vec<&NodeDetails<TNode>>, usize)> {
        let open_list = self.nodes.values().filter(|node| node.is_open);
        let node_indices = if count == 1 {
            let index = open_list.into_iter().min_by(sorting_function).map(|details| details.node.get_hash()).expect("no minimum");
            self.nodes.get_mut(&index).unwrap().is_open = false;
            vec![index]
        } else {
            let mut q_nodes: Vec<_> = open_list.into_iter().collect();
            q_nodes.sort_by(sorting_function);
            q_nodes.into_iter().take(count)
                .map(|details| details.node.get_hash())
                .collect::<Vec<_>>()
        };
        for &index in node_indices.iter() {
            self.nodes.get_mut(&index).unwrap().is_open = false;
        }
        let results: Vec<_> = node_indices.into_iter()
            .map(|i| self.nodes.get(&i).unwrap())
            .collect();
        if results.is_empty() {
            None
        } else {
            Some((results, self.nodes.values().filter(|n| n.is_open).count()))
        }
    }
}

pub fn a_star_search<TNode: AStarNode, TFunc: Fn(&TNode) -> Vec<Successor<TNode>> + Sync + Send, TFunc2: Fn(CurrentNodeDetails<TNode>) -> TNumber + Send + Sync>(start: TNode, end: &TNode, get_successors: TFunc, distance_function: TFunc2, options: Option<&AStarOptions>) -> Result<TNode> {
    let default_options = AStarOptions::default();
    let options = options.unwrap_or(&default_options);
    let mut node_list = NodeList::new(start);

    let mut i = 0usize;
    let n = if options.run_in_parallel {
        crate::num_cpus::get()
    } else { 1 };
    while let Some((nodes, remaining_list_len)) = node_list.get_next(n) {
        i += 1;
        print_debug(&nodes[..], remaining_list_len, i % (remaining_list_len / 10).max(1) == 0);

        let successors: Vec<NodeDetails<TNode>> = {
            if nodes.len() == 1 {
                let details = nodes.iter().cloned().next().unwrap();
                let (successors, done) = run_get_successors(details, end, &get_successors, &distance_function);
                if let Some(details) = done {
                    debug!("a_star took {} steps", i);
                    return Ok(make_results(details, node_list));
                }
                successors
            } else {
                let (successors, final_results) =
                    nodes.par_iter()
                        .map(|details| {
                            let get_successors = &get_successors;
                            let distance_function = &distance_function;
                            let (successors, done) = run_get_successors(details, end, get_successors, distance_function);
                            (successors, done)
                        })
                        .reduce(|| (Vec::new(), None), |(mut tot_successors, any_done): (Vec<NodeDetails<TNode>>, Option<NodeDetails<TNode>>), (successors, done): (Vec<NodeDetails<TNode>>, Option<NodeDetails<TNode>>)| {
                            if let Some(done_details) = &done {
                                if let Some(any_done_details) = &any_done {
                                    if done_details.g < any_done_details.g {
                                        return (tot_successors, done);
                                    }
                                } else {
                                    return (tot_successors, done);
                                }
                            }
                            tot_successors.extend(successors);
                            (tot_successors, any_done)
                        });
                if let Some(details) = final_results {
                    debug!("a_star took {} steps", i);
                    return Ok(make_results(details, node_list));
                }

                successors
            }
        };

        for details in successors {
            node_list.try_insert_successor(details);
        }
    }

    Err(AStarError::NoSolutionFound)
}

fn print_debug<TNode: AStarNode>(nodes: &[&NodeDetails<TNode>], list_len: usize, debug_level: bool) {
    if let Some(q_details) = nodes.iter().next() {
        if debug_level {
            debug!("got {:?}, list_len={}", q_details, list_len);
        } else {
            trace!("got {:?}, list_len={}", q_details, list_len);
        }
    }
}

fn run_get_successors<TNode: AStarNode, TFunc: Fn(&TNode) -> Vec<Successor<TNode>>, TFunc2: Fn(CurrentNodeDetails<TNode>) -> TNumber + Send + Sync>(parent: &NodeDetails<TNode>, end: &TNode, get_successors: &TFunc, distance_function: &TFunc2) -> (Vec<NodeDetails<TNode>>, Option<NodeDetails<TNode>>) {
    let successors = get_successors(&parent.node);
    let mut results: Vec<NodeDetails<TNode>> = Vec::with_capacity(successors.len());
    for Successor {
        node: successor,
        cost_to_move_here: distance
    } in successors {
        let to_current = parent.g + distance;

        if successor == *end {
            let details = NodeDetails::new_with(successor, to_current, 0, parent);
            return (vec![], Some(details));
        }

        let to_end = distance_function(CurrentNodeDetails {
            current_node: &successor,
            target_node: &end,
            cost_to_move_to_current: to_current,
        });
        let details = NodeDetails::new_with(successor, to_current, to_end, parent);
        results.push(details);
    }

    (results, None)
}

fn sorting_function<TNode: AStarNode>(a: &&NodeDetails<TNode>, b: &&NodeDetails<TNode>) -> Ordering {
    let c = a.f().cmp(&b.f());
    if c == Ordering::Equal {
        a.node.cmp(&b.node)
    } else {
        c
    }
}

fn make_results<TNode: AStarNode>(end: NodeDetails<TNode>, mut node_list: NodeList<TNode>) -> Vec<TNode> {
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
        Self { node, g, h, parent: None, is_open: true }
    }
    pub(crate) fn new_with(node: TNode, g: TNumber, h: TNumber, parent: &NodeDetails<TNode>) -> Self {
        Self { node, g, h, parent: Some(parent.node.get_hash()), is_open: true }
    }
    #[inline(always)]
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

        let options = AStarOptions::default().run_in_parallel();
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