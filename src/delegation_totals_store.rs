// RocksDB for DelegationTotals
use crate::gql::schema::delegations::{DelegationTotals, TotalDelegated};
use rocksdb::{Options, WriteBatch, DB};

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
    let mut batch = WriteBatch::default();

    let total_key = format!("{}_{}_total", public_key, epoch);
    let total_key_bytes = total_key.as_bytes();

    let count_key = format!("{}_{}_count", public_key, epoch);
    let count_key_bytes = count_key.as_bytes();

    let total_delegated_value = total_delegated.to_le_bytes();
    let count_delegates_value = count_delegates.to_le_bytes();

    batch.put(total_key_bytes, total_delegated_value);
    batch.put(count_key_bytes, count_delegates_value);

    db.write(batch)?;

    Ok(())
}

pub fn get_delegation_totals_from_db(
    db: &DB,
    public_key: &str,
    epoch: i32,
) -> Result<Option<DelegationTotals>, rocksdb::Error> {
    let combined_key = format!("{}_{}", public_key, epoch);
    let key = combined_key.as_bytes();

    let total_delegated_value = db.get(key)?;
    let count_delegates_value = db.get(key)?;

    if let (Some(total_delegated_bytes), Some(count_delegates_bytes)) = (
        total_delegated_value.as_ref(),
        count_delegates_value.as_ref(),
    ) {
        let total_delegated_bytes_array: [u8; 4] =
            total_delegated_bytes.as_slice().try_into().unwrap();
        let count_delegates_bytes_array: [u8; 4] =
            count_delegates_bytes.as_slice().try_into().unwrap();

        let total_delegated = f64::from_bits(u64::from_le_bytes(
            total_delegated_bytes[..].try_into().unwrap(),
        ));
        let count_delegates = i32::from_le_bytes(count_delegates_bytes_array);

        let delegation_totals = DelegationTotals {
            count_delegates,
            total_delegated: TotalDelegated(total_delegated),
        };
        Ok(Some(delegation_totals))
    } else {
        Ok(None)
    }
}
