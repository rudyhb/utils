use crate::a_star::helpers::GetHash;
use crate::common::Numeric;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use thiserror::Error;

pub trait Node: Hash + Send + Sync + Debug {}

impl<T: Node> CustomNode for T {
    const NODE_ID_AND_POSITION_HASH_SAME: bool = true;

    fn get_node_id(&self) -> u64 {
        self.get_hash()
    }

    fn get_position_hash(&self) -> u64 {
        self.get_hash()
    }
}

pub trait CustomNode: Send + Sync + Debug {
    const NODE_ID_AND_POSITION_HASH_SAME: bool;
    fn get_node_id(&self) -> u64;
    fn get_position_hash(&self) -> u64;
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Successor<TNode: CustomNode, TNumber: Numeric> {
    pub(crate) node: TNode,
    pub(crate) cost_to_move_here: TNumber,
}

pub struct ComputationResult<TNode: CustomNode, TNumber: Numeric> {
    pub shortest_path: Vec<TNode>,
    pub shortest_path_cost: TNumber,
}

pub struct CurrentNodeDetails<'a, TNode: CustomNode, TNumber: Numeric> {
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

pub(crate) struct NodeList<TNode: CustomNode, TNumber: Numeric> {
    pub(crate) candidate_nodes: HashMap<u64, NodeDetails<TNode, TNumber>>,
    pub(crate) node_history: HashMap<u64, NodeDetails<TNode, TNumber>>,
    pub(crate) cost_indexing: BTreeMap<TNumber, HashSet<u64>>,
    pub(crate) position_hash_to_min_accrued_cost: HashMap<u64, TNumber>,
}

#[derive(Eq, PartialEq)]
pub(crate) struct NodeDetails<TNode: CustomNode, TNumber: Numeric> {
    pub(crate) node: TNode,
    pub(crate) current_accrued_cost: TNumber,
    pub(crate) estimated_cost_to_goal: TNumber,
    pub(crate) parent: Option<u64>,
}
