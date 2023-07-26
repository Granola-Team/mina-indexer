// RocksDB for DelegationTotals
use crate::gql::schema::delegations::DelegationTotals;
use rocksdb::{Options, DB};

pub fn create_delegation_totals_db(path: &str) -> Result<DB, rocksdb::Error> {
    let mut options = Options::default();
    options.create_if_missing(true);
    DB::open(&options, path)
}

pub fn update_delegation_totals(
    db: &DB,
    public_key: &str,
    epoch: i32,
    total_delegated: i32,
    count_delegates: i32,
) -> Result<(), rocksdb::Error> {
    // placeholder for update delegation totals function
    unimplemented!()
}

pub fn get_delegation_totals_from_db(
    db: &DB,
    public_key: &str,
    epoch: i32,
) -> Result<Option<DelegationTotals>, rocksdb::Error> {
    // placeholder for function to retrieve delegation totals
    unimplemented!()
}
