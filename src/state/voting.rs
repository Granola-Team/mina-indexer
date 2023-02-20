use std::collections::HashSet;

use super::State;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum Status {
    Invalid,
    Propose,
    Active,
    Complete,
    Unknown,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct Mip {
    pub name: String,
    pub epoch: u32,
    pub start_slot: u32,
    pub end_slot: u32,
    pub yes_stake: u64,
    pub no_stake: u64,
    pub status: Status,
}

#[derive(Debug)]
pub struct Voting {
    pub propose_mips: HashSet<Mip>,
    pub active_mips: HashSet<Mip>,
    pub complete_mips: HashSet<Mip>,
}

impl Mip {
    pub fn new(name: String, epoch: u32, start_slot: u32, end_slot: u32) -> Self {
        Mip {
            name,
            epoch,
            start_slot,
            end_slot,
            yes_stake: 0,
            no_stake: 0,
            status: Status::Unknown,
        }
    }

    pub fn add_stake(&mut self, vote: bool, amount: u64) {
        if vote {
            self.yes_stake += amount;
        } else {
            self.no_stake += amount;
        }
    }

    pub fn update_status(&mut self, state: &State) {
        let status = state.voting_mip_status(self);
        self.status = status;
    }
}

impl Voting {
    pub fn new() -> Self {
        Voting {
            propose_mips: HashSet::new(),
            active_mips: HashSet::new(),
            complete_mips: HashSet::new(),
        }
    }

    pub fn insert(&mut self, mip: &mut Mip, state: &State) {
        mip.update_status(state);
        match state.voting_mip_status(mip) {
            Status::Active => {
                self.active_mips.insert(*mip);
            }
            Status::Complete => {
                self.complete_mips.insert(*mip);
            }
            Status::Propose => {
                self.propose_mips.insert(*mip);
            }
            _ => {}
        }
    }
}
