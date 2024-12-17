use crate::a_star::helpers::GetHash;
use crate::a_star::models::{NodeDetails, NodeList};
use crate::a_star::{Error, Node, Result, Successor};
use crate::common::Numeric;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter};

impl<TNode: Node, TNumber: Numeric> Successor<TNode, TNumber> {
    pub fn new(node: TNode, cost_to_move_here: TNumber) -> Self {
        Self {
            node,
            cost_to_move_here,
        }
    }
}

impl<TNode: Node, TNumber: Numeric> NodeList<TNode, TNumber> {
    pub(crate) fn new(start: TNode) -> Self {
        let mut result = Self {
            nodes: Default::default(),
        };
        let hash = start.get_hash();
        result.nodes.insert(
            hash,
            NodeDetails::new(start, TNumber::default(), TNumber::default()),
        );
        result
    }
    pub(crate) fn try_insert_successor(&mut self, details: NodeDetails<TNode, TNumber>) {
        let hash = details.node.get_hash();
        if let Some(existing) = self.nodes.get(&hash) {
            if existing.f() <= details.f() {
                return;
            }
        }
        self.nodes.insert(hash, details);
    }
    pub(crate) fn get_next(&mut self) -> Result<(&NodeDetails<TNode, TNumber>, usize)> {
        let index = self
            .nodes
            .values()
            .filter(|node| node.is_open)
            .min()
            .map(|details| details.node.get_hash())
            .ok_or(Error::NoSolutionFound)?;
        self.nodes.get_mut(&index).unwrap().is_open = false;
        let result = self.nodes.get(&index).ok_or(Error::UnexpectedError)?;
        Ok((result, self.nodes.values().filter(|n| n.is_open).count()))
    }
}

impl<T: Node, TNumber: Numeric> PartialOrd for NodeDetails<T, TNumber> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let c = self.f().cmp(&other.f());
        if c == Ordering::Equal {
            Some(self.node.cmp(&other.node))
        } else {
            Some(c)
        }
    }
}

impl<T: Node, TNumber: Numeric> Ord for NodeDetails<T, TNumber> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl<TNode: Node, TNumber: Numeric> Debug for NodeDetails<TNode, TNumber> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut val = format!("{:?}", self.node);
        if val.len() > 130 {
            val = format!("{}..{}", &val[..65], &val[val.len() - 65..])
        }
        write!(f, "q {} with g={}, h={}", val, self.g, self.h)
    }
}

impl<TNode: Node, TNumber: Numeric> NodeDetails<TNode, TNumber> {
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
        parent: &NodeDetails<TNode, TNumber>,
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
