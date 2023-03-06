use std::collections::HashSet;

use crate::block::BlockHash;

use self::branch::Branch;

pub mod ledger;
// pub mod best_tip;
// pub mod block;
pub mod branch;
// pub mod store;
// pub mod voting;

// use self::head::Head;
// use self::block::{Block, BlockHash};
// use self::branch::{Branches, Leaves};
// use self::store::Store;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Head {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Store {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Status {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct DanglingBranches {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct StateUpdate {}

pub type RefLog = Vec<StateUpdate>;

#[derive(Debug, PartialEq, Eq)]
pub struct State {
    pub root: BlockHash,
    pub head: Head,
    pub branches: HashSet<Branch>,
    pub store: Store,
    pub status: Status,
    pub dangling: DanglingBranches, // HashSet<Branch>
    pub reflog: RefLog,             // Vec<StateUpdate>
}
