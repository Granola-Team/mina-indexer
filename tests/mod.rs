mod block;
mod canonicity;
mod command;
mod event;
mod state;

pub mod helpers {
    /// Removes the dir if it exists, creates a fesh dir
    pub fn setup_new_db_dir(db_path: &str) -> std::path::PathBuf {
        let mut store_dir = std::env::temp_dir();
        store_dir.push(db_path);

        if store_dir.exists() {
            std::fs::remove_dir_all(store_dir.clone()).unwrap();
        }
        store_dir
    }
}
