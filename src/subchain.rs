use petgraph::{graph::NodeIndex, visit::EdgeRef, Graph};
use std::{collections::HashMap, rc::Rc};

use crate::Block;

type StateHash = String;
type BlockReference = std::rc::Rc<Block>;
type GraphIndices = Vec<NodeIndex<u32>>;
type StateHashGraphIndexMap = HashMap<StateHash, NodeIndex<u32>>;
type SubchainGraph = Graph<BlockReference, StateHash>;

pub struct SubchainIndexer {
    root_indices: GraphIndices,
    block_indices: StateHashGraphIndexMap,
    chain_graph: SubchainGraph,
}

impl SubchainIndexer {
    pub fn new() -> Self {
        Self {
            root_indices: Vec::new(),
            block_indices: HashMap::new(),
            chain_graph: Graph::new(),
        }
    }

    pub fn as_dot(&self) -> String {
        use petgraph::dot::Dot;
        Dot::new(&self.chain_graph).to_string()
    }

    pub fn add_block(&mut self, block: Block) {
        let block_ref = Rc::new(block);
        let index = self.chain_graph.add_node(block_ref.clone());
        self.block_indices
            .insert(block_ref.state_hash.clone(), index);
        if let Some(prev_index) = self
            .block_indices
            .get(&block_ref.protocol_state.previous_state_hash)
        {
            self.chain_graph.add_edge(
                prev_index.clone(),
                index,
                block_ref.protocol_state.previous_state_hash.clone(),
            );
        } else {
            self.root_indices.push(index);
        }
    }

    pub fn longest_chain(&self) -> Vec<BlockReference> {
        let mut longest_chain = Vec::new();
        for index in self.root_indices.iter() {
            let next_chain = longest_chain_rec(&self.chain_graph, *index);
            if longest_chain.len() < next_chain.len() {
                longest_chain = next_chain;
            }
        }
        longest_chain
    }
}

fn longest_chain_rec(
    graph: &Graph<BlockReference, StateHash>,
    root_index: NodeIndex<u32>,
) -> Vec<BlockReference> {
    let mut longest_chain = Vec::new();
    if let Some(block) = graph.node_weight(root_index) {
        longest_chain.push(block.clone());
        let mut longest_tail = Vec::new();
        for edge in graph.edges(root_index) {
            let next_tail = longest_chain_rec(graph, edge.target());
            if longest_tail.len() < next_tail.len() {
                longest_tail = next_tail;
            }
        }
        longest_chain.append(&mut longest_tail);
    }

    longest_chain
}
