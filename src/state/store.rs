use std::collections::HashSet;

use super::block::Block;
use super::ledger::Ledger;
use super::voting::Voting;

type Blocks = HashSet<Block>;
type Ledgers = HashSet<Ledger>;

#[derive(Debug, PartialEq, Eq)]
pub struct Store {
    pub blocks: Blocks,
    pub ledgers: Ledgers,
    pub voting: Voting,
}

impl Store {
    pub fn new() -> Self {
        Store {
            blocks: HashSet::new(),
            ledgers: HashSet::new(),
            voting: Voting::new(),
        }
    }

    // TODO add Store impl
}

impl std::hash::Hash for Store {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for block in self.blocks.iter() {
            block.hash(state);
        }
        for ledger in self.ledgers.iter() {
            ledger.hash(state);
        }
        self.voting.hash(state);
    }
}
