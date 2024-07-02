use super::{
    fixed_keys::FixedKeys,
    version::{IndexerStoreVersion, VersionStore},
    IndexerStore,
};
use log::trace;

impl VersionStore for IndexerStore {
    /// Set db version with env var `GIT_COMMIT_HASH`
    fn set_db_version_with_git_commit(
        &self,
        major: u32,
        minor: u32,
        patch: u32,
    ) -> anyhow::Result<()> {
        let version = IndexerStoreVersion {
            major,
            minor,
            patch,
            ..Default::default()
        };
        trace!("Setting database version: {version:#?}");
        if self
            .database
            .get(Self::INDEXER_STORE_VERSION_KEY)?
            .is_none()
        {
            self.database.put(
                Self::INDEXER_STORE_VERSION_KEY,
                serde_json::to_vec(&version)?,
            )?;
        }
        Ok(())
    }

    /// Get db version
    fn get_db_version(&self) -> anyhow::Result<IndexerStoreVersion> {
        trace!("Getting database version");
        Ok(self
            .database
            .get(Self::INDEXER_STORE_VERSION_KEY)?
            .map(|bytes| serde_json::from_slice(&bytes).expect("db version bytes"))
            .expect("db version some"))
    }
}
