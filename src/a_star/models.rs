use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use thiserror::Error;
use crate::common::Numeric;

pub trait Node: Hash + Eq + PartialEq + Ord + PartialOrd + Send + Sync + Debug {}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Successor<TNode: Node, TNumber: Numeric> {
    pub(crate) node: TNode,
    pub(crate) cost_to_move_here: TNumber,
}

pub struct ComputationResult<TNode: Node, TNumber: Numeric> {
    pub shortest_path: Vec<TNode>,
    pub shortest_path_cost: TNumber,
}

pub struct CurrentNodeDetails<'a, TNode: Node, TNumber: Numeric> {
    pub current_node: &'a TNode,
    pub target_node: &'a TNode,
    pub cost_to_move_to_current: TNumber,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("No solution found")]
    NoSolutionFound,
    #[error("An unexpected error occurred")]
    UnexpectedError,
    #[error("Iteration limit exceeded")]
    IterLimitExceeded,
}

pub(crate) struct NodeList<TNode: Node, TNumber: Numeric> {
    pub(crate) candidate_nodes: HashMap<u64, NodeDetails<TNode, TNumber>>,
    pub(crate) node_history: HashMap<u64, NodeDetails<TNode, TNumber>>,
}

#[derive(Eq, PartialEq)]
pub(crate) struct NodeDetails<TNode: Node, TNumber: Numeric> {
    pub(crate) node: TNode,
    pub(crate) current_accrued_cost: TNumber,
    pub(crate) estimated_cost_to_goal: TNumber,
    pub(crate) parent: Option<u64>,
}
