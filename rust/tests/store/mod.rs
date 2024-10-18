use anyhow::Result;

#[cfg(test)]
mod tests {
    use mina_indexer::{snark_work::store::SnarkStore, store::IndexerStore};

    use crate::helpers::setup_new_db_dir;

    use super::*;

    fn create_indexer_store() -> Result<IndexerStore> {
        let test_dir = setup_new_db_dir("indexer-store-tests")?;
        let store = IndexerStore::new(test_dir.path())?;
        Ok(store)
    }

    #[test]
    fn test_incr_snarks_total_non_canonical_count() -> Result<()> {
        let indexer = create_indexer_store()?;

        indexer.increment_snarks_total_non_canonical_count()?;
        assert_eq!(indexer.get_snarks_total_canonical_count()?, 1);

        indexer.increment_snarks_total_non_canonical_count()?;
        assert_eq!(indexer.get_snarks_total_canonical_count()?, 2);

        Ok(())
    }

    #[test]
    fn test_incr_snarks_total_canonical_count() -> Result<()> {
        let indexer = create_indexer_store()?;

        indexer.increment_snarks_total_canonical_count()?;
        assert_eq!(indexer.get_snarks_total_canonical_count()?, 1);

        indexer.increment_snarks_total_canonical_count()?;
        assert_eq!(indexer.get_snarks_total_canonical_count()?, 2);

        Ok(())
    }
}
