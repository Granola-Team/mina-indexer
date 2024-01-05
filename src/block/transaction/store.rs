///
pub trait TransactionStore {
    /// Add transactions from the block indexed on
    /// public keys, transaction hash, and state hashes
    fn add_transactions(block: &PrecomputedBlock) -> anyhow::Result<()>;
}
