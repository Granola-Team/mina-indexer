use crate::block::{self, parser::BlockParser, precomputed::PrecomputedBlock};
use async_priority_channel as priority;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use thiserror::Error;
use tokio::sync::{
    mpsc,
    watch::{self, Sender},
};
use tracing::{debug, error, instrument};
use watchexec::{error::RuntimeError, sources::fs::worker, Config};
use watchexec_events::{
    filekind::{
        CreateKind,
        FileEventKind::{Create, Modify},
    },
    Event, Priority, Tag,
};

#[derive(Debug, Clone, Hash, Serialize, Deserialize, Error)]
pub enum FilesystemReceiverError {
    WatchTargetIsNotADirectory(PathBuf),
    WorkerRuntimeError(String),
}

pub type FilesystemReceiverResult<T> = std::result::Result<T, FilesystemReceiverError>;

pub struct FilesystemReceiver {
    parsers: Vec<BlockParser>,
    paths_added: HashSet<PathBuf>,
    worker_command_sender: Sender<Config>,
    worker_event_receiver: priority::Receiver<Event, Priority>,
    worker_error_receiver: mpsc::Receiver<RuntimeError>,
}

impl FilesystemReceiver {
    #[instrument]
    pub async fn new(event_capacity: u64, error_capacity: usize) -> FilesystemReceiverResult<Self> {
        debug!("initializing new filesystem receiver");
        let (ev_s, worker_event_receiver) = priority::bounded(event_capacity);
        let (er_s, worker_error_receiver) = mpsc::channel(error_capacity);

        let config = Config::default();
        config.pathset(["."]);

        let (worker_command_sender, _) = watch::channel(config.clone());

        tokio::spawn(async {
            debug!("spawning filesystem watcher worker");
            worker(config.into(), er_s, ev_s)
                .await
                .expect("should not crash");
        });

        Ok(Self {
            parsers: vec![],
            paths_added: HashSet::new(),
            worker_command_sender,
            worker_event_receiver,
            worker_error_receiver,
        })
    }

    pub fn watched_directories(&self) -> Vec<PathBuf> {
        self.parsers
            .iter()
            .map(|parser| parser.blocks_dir.clone())
            .collect()
    }

    pub fn load_directory(
        &mut self,
        directory: impl AsRef<Path>,
    ) -> Result<(), FilesystemReceiverError> {
        debug!(
            "loading directory {} into FilesystemReceiver",
            directory.as_ref().display()
        );

        if !directory.as_ref().is_dir() {
            return Err(FilesystemReceiverError::WatchTargetIsNotADirectory(
                PathBuf::from(directory.as_ref()),
            ));
        }

        debug!("sending command to worker with new working data");
        let mut watched_directories = self.watched_directories();
        watched_directories.push(PathBuf::from(directory.as_ref()));

        let working_data = Config::default();
        working_data.pathset(watched_directories);

        self.worker_command_sender.send_replace(working_data);

        Ok(())
    }
}

#[async_trait]
impl super::BlockReceiver for FilesystemReceiver {
    async fn recv_block(&mut self) -> anyhow::Result<Option<PrecomputedBlock>> {
        loop {
            tokio::select! {
                error_fut = self.worker_error_receiver.recv() => {
                    if let Some(error) = error_fut {
                        return Err(error)
                            .map_err(|e| FilesystemReceiverError::WorkerRuntimeError(e.to_string()).into()
                        );
                    }
                    return Ok(None);
                },
                event_fut = self.worker_event_receiver.recv() => {
                    if let Ok((event, _priority)) = event_fut {
                        let mut tags = event
                        .tags
                        .iter();
                        if tags.any(|signal|
                                matches!(signal, Tag::Path { .. })
                                ||
                                matches!(signal, Tag::FileEventKind(Modify(_)))
                                ||
                                matches!(signal, Tag::FileEventKind(Create(CreateKind::File)))
                            )
                        {
                            for tag in tags {
                                if let Tag::Path { path, .. } = tag {
                                    if self.paths_added.len() == 100 {
                                        self.paths_added.clear()
                                    }
                                    if block::is_valid_block_file(path) && !self.paths_added.contains(path) {
                                        self.paths_added.insert(path.clone());
                                        match PrecomputedBlock::parse_file(path.as_path()) {
                                            Ok(block) => return Ok(Some(block)),
                                            _ => {
                                                error!("Cannot parse block at {}", path.display());
                                                continue;
                                            },
                                        }
                                    }
                                } else {
                                    continue;
                                }
                            }
                        }
                    }
                    continue;
                }
            }
        }
    }
}

impl std::fmt::Display for FilesystemReceiverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilesystemReceiverError::WatchTargetIsNotADirectory(directory) => {
                f.write_str(&format!(
                    "cannot watch {} for new blocks, it is not a directory",
                    directory.display()
                ))
            }
            FilesystemReceiverError::WorkerRuntimeError(runtime_error) => f.write_str(&format!(
                "encountered an error while running the filesystem worker: {}",
                runtime_error
            )),
        }
    }
}
