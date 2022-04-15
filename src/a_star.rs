use std::cmp::Ordering;
use std::collections::{HashMap};
use std::fmt::Debug;
use std::hash::Hash;

type TNumber = i32;

pub trait AStarNode: Hash + Eq + PartialEq + Clone + Ord + PartialOrd + Send + std::marker::Sync + Debug {}

pub struct AStarOptions {
    pub print_debug: bool,
    pub print_current_val: bool,
    pub print_every: Option<usize>,
}

const DEFAULT_OPTIONS: AStarOptions = AStarOptions {
    print_debug: false,
    print_current_val: false,
    print_every: None,
};

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

pub fn a_star_search<TNode: AStarNode, TFunc: Fn(&TNode) -> Vec<Successor<TNode>>, TFunc2: Fn(CurrentNodeDetails<TNode>) -> TNumber + Send + std::marker::Sync>(start: TNode, end: &TNode, get_successors: TFunc, distance_function: TFunc2, options: Option<&AStarOptions>) -> Option<Vec<TNode>> {
    let options = options.unwrap_or(&DEFAULT_OPTIONS);
    let mut closed_list: HashMap<TNode, NodeDetails<TNode>> = Default::default();
    let mut open_list: HashMap<TNode, NodeDetails<TNode>> = Default::default();
    open_list.insert(start, NodeDetails::new(0, 0));

    let mut i = 0usize;
    while !open_list.is_empty() {
        i += 1;

        let (q_node, q_details, list_len) = {
            let q_node = open_list.iter().min_by(|(a_node, a), (b_node, b)| {
                let c = a.f().cmp(&b.f());
                if c == Ordering::Equal {
                    a_node.cmp(&b_node)
                } else {
                    c
                }
            }).map(|(key, _)| key).cloned().expect("no minimum");
            let q_details = open_list.remove(&q_node).unwrap();
            (q_node, q_details, open_list.len())
        };

        if options.print_debug {
            let print = if let Some(every) = options.print_every {
                i % every == 0
            } else {
                true
            };
            if print {
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

        let successors = get_successors(&q_node);
        for Successor {
            node: successor,
            cost_to_move_here: distance
        } in successors {
            let to_current = q_details.g + distance;

            if successor == *end {
                let details = NodeDetails::new_with(to_current, 0, q_node.clone(), q_details.clone());
                return Some(make_results(successor, details));
            }

            let to_end = distance_function(CurrentNodeDetails {
                current_node: &successor,
                target_node: &end,
                cost_to_move_to_current: to_current,
            });
            let details = NodeDetails::new_with(to_current, to_end, q_node.clone(), q_details.clone());

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

        closed_list.insert(q_node, q_details);
    }
    None
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