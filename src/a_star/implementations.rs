use crate::a_star::models::{CustomNode, NodeDetails, NodeList};
use crate::a_star::{Error, Result, Successor};
use crate::common::Numeric;
use std::fmt::{Debug, Formatter};

impl<TNode: CustomNode, TNumber: Numeric> Successor<TNode, TNumber> {
    pub fn new(node: TNode, cost_to_move_here: TNumber) -> Self {
        Self {
            node,
            cost_to_move_here,
        }
    }
}

impl<TNode: CustomNode, TNumber: Numeric> NodeList<TNode, TNumber> {
    pub(crate) fn new(start: TNode) -> Self {
        let mut result = Self {
            candidate_nodes: Default::default(),
            node_history: Default::default(),
            cost_indexing: Default::default(),
            position_hash_to_min_accrued_cost: Default::default(),
        };
        result.insert_candidate(
            NodeDetails::new(start, TNumber::default(), TNumber::default()),
            None,
            None,
        );
        result
    }
    fn insert_candidate(
        &mut self,
        node: NodeDetails<TNode, TNumber>,
        id: Option<u64>,
        position: Option<u64>,
    ) {
        let id = id.unwrap_or_else(|| node.node.get_node_id());
        let estimated_cost = node.sum_accrued_plus_estimated_cost();
        let accrued_cost = node.current_accrued_cost;
        let position = position.unwrap_or_else(|| {
            if TNode::NODE_ID_AND_POSITION_HASH_SAME {
                id
            } else {
                node.node.get_position_hash()
            }
        });
        self.cost_indexing
            .entry(estimated_cost)
            .or_default()
            .insert(id);
        self.candidate_nodes.insert(id, node);
        self.position_hash_to_min_accrued_cost
            .entry(position)
            .and_modify(|existing| *existing = accrued_cost.min(*existing))
            .or_insert(accrued_cost);
    }
    fn remove_candidate(&mut self, index: u64) -> NodeDetails<TNode, TNumber> {
        let node = self
            .candidate_nodes
            .remove(&index)
            .expect("inconsistency between cost indexing and candidate nodes");
        let cost = node.sum_accrued_plus_estimated_cost();
        let indices = self.cost_indexing.get_mut(&cost).unwrap();
        indices.remove(&index);
        if indices.is_empty() {
            self.cost_indexing.remove(&cost);
        }
        node
    }
    pub(crate) fn try_insert_successor(&mut self, details: NodeDetails<TNode, TNumber>) {
        let position = details.node.get_position_hash();
        let accrued_cost = details.current_accrued_cost;

        let compare = if TNode::NODE_ID_AND_POSITION_HASH_SAME {
            TNumber::le
        } else {
            TNumber::lt
        };

        if let Some(&existing) = self.position_hash_to_min_accrued_cost.get(&position) {
            if compare(&existing, &accrued_cost) {
                return;
            }
        }

        let id = if TNode::NODE_ID_AND_POSITION_HASH_SAME {
            position
        } else {
            details.node.get_node_id()
        };

        if let Some(existing) = self.candidate_nodes.get(&id) {
            if TNode::NODE_ID_AND_POSITION_HASH_SAME
                && existing.current_accrued_cost <= accrued_cost
            {
                return;
            }
            self.remove_candidate(id);
        } else if TNode::NODE_ID_AND_POSITION_HASH_SAME {
            if let Some(existing) = self.node_history.get(&id) {
                if existing.current_accrued_cost <= accrued_cost {
                    return;
                }
            }
        }

        self.insert_candidate(details, Some(id), Some(position));
    }
    pub(crate) fn get_next(&mut self) -> Result<(&NodeDetails<TNode, TNumber>, usize)> {
        let index = self
            .cost_indexing
            .first_key_value()
            .and_then(|(_, id)| id.iter().next().copied())
            .ok_or(Error::NoSolutionFound)?;
        let node = self.remove_candidate(index);
        self.node_history.insert(index, node);
        let result = self
            .node_history
            .get(&index)
            .ok_or(Error::UnexpectedError)?;
        Ok((result, self.candidate_nodes.len()))
    }
}

impl<TNode: CustomNode, TNumber: Numeric> Debug for NodeDetails<TNode, TNumber> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut val = format!("{:?}", self.node);
        if val.len() > 130 {
            val = format!("{}..{}", &val[..65], &val[val.len() - 65..])
        }
        write!(
            f,
            "node {} with accrued={}, estimated_to_goal={}",
            val, self.current_accrued_cost, self.estimated_cost_to_goal
        )
    }
}

impl<TNode: CustomNode, TNumber: Numeric> NodeDetails<TNode, TNumber> {
    pub(crate) fn new(
        node: TNode,
        current_accrued_cost: TNumber,
        estimated_cost_to_goal: TNumber,
    ) -> Self {
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
            parent: Some(parent.node.get_node_id()),
        }
    }
    #[inline(always)]
    pub(crate) fn sum_accrued_plus_estimated_cost(&self) -> TNumber {
        self.current_accrued_cost + self.estimated_cost_to_goal
    }
}
