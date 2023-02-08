use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use anyhow::Result;
use log::*;
use thiserror::Error;

use crate::a_star::helpers::GetHash;
use crate::a_star::{AStarNode, TNumber};

pub struct Successor<TNode: AStarNode> {
    pub(crate) node: TNode,
    pub(crate) cost_to_move_here: TNumber,
}

pub struct AStarResult<TNode: AStarNode> {
    pub shortest_path: Vec<TNode>,
    pub shortest_path_cost: i32,
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
    #[error("Iteration limit exceeded")]
    IterLimitExceeded,
}

pub(crate) struct NodeList<TNode: AStarNode> {
    pub(crate) nodes: HashMap<u64, NodeDetails<TNode>>,
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
            .min()
            .map(|details| details.node.get_hash())
            .ok_or(AStarError::NoSolutionFound)?;
        self.nodes.get_mut(&index).unwrap().is_open = false;
        let result = self.nodes.get(&index).ok_or(AStarError::UnexpectedError)?;
        Ok((result, self.nodes.values().filter(|n| n.is_open).count()))
    }
}

#[derive(Eq, PartialEq)]
pub(crate) struct NodeDetails<TNode: AStarNode> {
    pub(crate) node: TNode,
    pub(crate) is_open: bool,
    pub(crate) g: TNumber,
    pub(crate) h: TNumber,
    pub(crate) parent: Option<u64>,
}

impl<T: AStarNode> PartialOrd for NodeDetails<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let c = self.f().cmp(&other.f());
        if c == Ordering::Equal {
            Some(self.node.cmp(&other.node))
        } else {
            Some(c)
        }
    }
}

impl<T: AStarNode> Ord for NodeDetails<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
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
