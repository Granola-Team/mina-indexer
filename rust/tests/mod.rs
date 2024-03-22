mod block;
mod canonicity;
mod command;
mod event;
#[cfg(all(test, feature = "mina_rs"))]
mod protocol;
mod snark_work;
mod state;

pub mod helpers {
    /// Sets up a new temp dir, deleted when it goes out of scope
    pub fn setup_new_db_dir(prefix: &str) -> anyhow::Result<tempfile::TempDir> {
        let store_dir = tempfile::TempDir::with_prefix(prefix)?;
        if store_dir.path().exists() {
            std::fs::remove_dir_all(store_dir.path())?;
        }
        Ok(store_dir)
    }
}
