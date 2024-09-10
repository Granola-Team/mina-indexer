use crate::ledger::account::Amount;
use csv::Reader;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Record {
    slot: u32,
    locked: u64,
}

#[derive(Default)]
pub struct LockedBalances {
    locked: HashMap<u32, Amount>,
}

pub const LOCKED_BALANCES_CONTENTS: &str = include_str!("../../../data/locked.csv");

impl LockedBalances {
    pub fn new() -> anyhow::Result<Self> {
        let mut locked = HashMap::new();

        let mut rdr = Reader::from_reader(LOCKED_BALANCES_CONTENTS.as_bytes());
        for result in rdr.deserialize() {
            let record: Record = result?;
            locked.insert(record.slot, Amount::new(record.locked * 1_000_000_000_u64));
        }

        Ok(LockedBalances { locked })
    }

    pub fn get_locked_amount(&self, global_slot: u32) -> Option<Amount> {
        self.locked.get(&global_slot).copied()
    }
}
