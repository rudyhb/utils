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
            candidate_nodes: Default::default(),
            node_history: Default::default(),
        };
        let hash = start.get_hash();
        result.candidate_nodes.insert(
            hash,
            NodeDetails::new(start, TNumber::default(), TNumber::default()),
        );
        result
    }
    pub(crate) fn try_insert_successor(&mut self, details: NodeDetails<TNode, TNumber>) {
        let hash = details.node.get_hash();
        if [&self.node_history, &self.candidate_nodes]
            .into_iter().any(|nodes| {
            if let Some(existing) = nodes.get(&hash) {
                if existing.sum_accrued_plus_estimated_cost() <= details.sum_accrued_plus_estimated_cost() {
                    return true;
                }
            }
            false
        }) {
            return;
        }
        self.candidate_nodes.insert(hash, details);
    }
    pub(crate) fn get_next(&mut self) -> Result<(&NodeDetails<TNode, TNumber>, usize)> {
        let index = self
            .candidate_nodes
            .values()
            .min()
            .map(|details| details.node.get_hash())
            .ok_or(Error::NoSolutionFound)?;
        let node = self.candidate_nodes.remove(&index).unwrap();
        self.node_history.insert(index, node);
        let result = self.node_history.get(&index).ok_or(Error::UnexpectedError)?;
        Ok((result, self.candidate_nodes.len()))
    }
}

impl<T: Node, TNumber: Numeric> PartialOrd for NodeDetails<T, TNumber> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let c = self.sum_accrued_plus_estimated_cost().cmp(&other.sum_accrued_plus_estimated_cost());
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
        write!(f, "node {} with accrued={}, estimated_to_goal={}", val, self.current_accrued_cost, self.estimated_cost_to_goal)
    }
}

impl<TNode: Node, TNumber: Numeric> NodeDetails<TNode, TNumber> {
    pub(crate) fn new(node: TNode, current_accrued_cost: TNumber, estimated_cost_to_goal: TNumber) -> Self {
        Self {
            node,
            current_accrued_cost,
            parent: None,
            estimated_cost_to_goal,
        }
    }
    pub(crate) fn new_with_parent(
        node: TNode,
        current_accrued_cost: TNumber,
        estimated_cost_to_goal: TNumber,
        parent: &NodeDetails<TNode, TNumber>,
    ) -> Self {
        Self {
            node,
            current_accrued_cost,
            estimated_cost_to_goal,
            parent: Some(parent.node.get_hash()),
        }
    }
    #[inline(always)]
    pub(crate) fn sum_accrued_plus_estimated_cost(&self) -> TNumber {
        self.current_accrued_cost + self.estimated_cost_to_goal
    }
}
