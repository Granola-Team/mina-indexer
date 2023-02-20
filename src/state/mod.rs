pub mod ledger;
// pub mod best_tip;
// pub mod block;
// pub mod branch;
// pub mod store;
// pub mod voting;

// use self::head::Head;
// use self::block::{Block, BlockHash};
// use self::branch::{Branches, Leaves};
// use self::store::Store;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Head {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Block {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct BlockHash {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Branch {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Branches {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Leaves {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Store {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Status {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct DanglingBranches {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct StateUpdate {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct RefLog {}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct State {
    pub root: BlockHash,
    pub head: Head,
    pub leaves: Leaves,
    pub branches: Branches,
    pub store: Store,
    pub status: Status,
    pub dangling: DanglingBranches, // HashSet<Branch>
    pub reflog: RefLog,             // Vec<StateUpdate>
}
