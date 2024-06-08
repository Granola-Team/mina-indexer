use crate::constants::GIT_COMMIT_HASH;
use serde::{Deserialize, Serialize};

pub trait VersionStore {
    /// Set db version
    fn set_db_version_with_git_commit(
        &self,
        major: u32,
        minor: u32,
        patch: u32,
    ) -> anyhow::Result<()>;

    /// Get db version
    fn get_db_version(&self) -> anyhow::Result<IndexerStoreVersion>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerStoreVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub git_commit_sha: String,
}

impl IndexerStoreVersion {
    pub const MAJOR: u32 = 0;
    pub const MINOR: u32 = 1;
    pub const PATCH: u32 = 0;
}

impl std::default::Default for IndexerStoreVersion {
    fn default() -> Self {
        Self {
            major: Self::MAJOR,
            minor: Self::MINOR,
            patch: Self::PATCH,
            git_commit_sha: GIT_COMMIT_HASH.to_string(),
        }
    }
}

impl std::fmt::Display for IndexerStoreVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}-{}",
            self.major, self.minor, self.patch, self.git_commit_sha
        )
    }
}
