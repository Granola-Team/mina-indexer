use std::path::{Path, PathBuf};

use crossbeam_channel::bounded;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::{debug, error, info, warn};

use crate::block;

/// Watches a directory listening for when valid precomputed blocks are created and signals downstream
pub fn watch_directory_for_blocks<P: AsRef<Path>>(
    watch_dir: P,
    sender: crossbeam_channel::Sender<PathBuf>,
) -> notify::Result<()> {
    info!("Starting block watcher thread..");
    let (tx, rx) = bounded(4096);
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    watcher.watch(watch_dir.as_ref(), RecursiveMode::NonRecursive)?;
    info!(
        "Watching for new blocks in directory: {:?}",
        watch_dir.as_ref()
    );
    for res in rx {
        match res {
            Ok(event) => {
                if let EventKind::Create(notify::event::CreateKind::File)
              // Because of the way gsutil does resumable downloads,
              // it first creates a file with a suffix .gstmp then
              // when it's finished downloading, it will rename the
              // file to the proper extension.
              | EventKind::Modify(notify::event::ModifyKind::Name(_)) = event.kind
              {
                  for path in event.paths {
                      if block::is_valid_block_file(&path) {
                          debug!("Valid precomputed block file: {}", path.display());
                          if let Err(e) = sender.send(path) {
                              error!("Unable to send path downstream. {}", e);
                          }
                      } else {
                          warn!("Invalid precomputed block file: {}", path.display());
                      }
                  }
              }
            }
            Err(error) => error!("Error: {error:?}"),
        }
    }
    Ok(())
}
