use async_priority_channel as priority;
use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::{
    mpsc,
    watch::{self, Sender},
};
use tracing::{debug, info, instrument};
use watchexec::{
    error::RuntimeError,
    event::{
        filekind::{
            CreateKind,
            FileEventKind::{Create, Modify},
        },
        Event, Priority, Tag,
    },
    fs::{worker, WorkingData},
};

use crate::block::{
    parse_file, parser::filesystem::FilesystemParser, precomputed::PrecomputedBlock,
};

#[derive(Debug, Clone, Hash, Serialize, Deserialize, Error)]
pub enum FilesystemReceiverError {
    WatchTargetIsNotADirectory(PathBuf),
    WorkerRuntimeError(String),
}
pub type FilesystemReceiverResult<T> = std::result::Result<T, FilesystemReceiverError>;
pub struct FilesystemReceiver {
    parsers: Vec<FilesystemParser>,
    worker_command_sender: Sender<WorkingData>,
    worker_event_receiver: priority::Receiver<Event, Priority>,
    worker_error_receiver: mpsc::Receiver<RuntimeError>,
}

impl FilesystemReceiver {
    #[instrument]
    pub async fn new(
        event_capacity: usize,
        error_capacity: usize,
    ) -> FilesystemReceiverResult<Self> {
        info!("initializing new filesystem receiver");
        let (ev_s, worker_event_receiver) = priority::bounded(event_capacity);
        let (er_s, worker_error_receiver) = mpsc::channel(error_capacity);
        let (worker_command_sender, wd_r) = watch::channel(WorkingData::default());

        tokio::spawn(async {
            debug!("spawning filesystem watcher worker");
            worker(wd_r, er_s, ev_s).await.expect("should not crash");
        });

        let parsers = vec![];
        Ok(Self {
            parsers,
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
        info!(
            "loading directory {} into FilesystemReceiver",
            directory.as_ref().display()
        );

        if !directory.as_ref().is_dir() {
            return Err(FilesystemReceiverError::WatchTargetIsNotADirectory(
                PathBuf::from(directory.as_ref()),
            ));
        }

        debug!("sending command to worker with new working data");
        let mut working_data = WorkingData::default();
        let mut watched_directories = self.watched_directories();
        watched_directories.push(PathBuf::from(directory.as_ref()));
        working_data.pathset = watched_directories
            .iter()
            .map(|path_buf| {
                let path: &Path = path_buf.as_ref();
                path.into()
            })
            .collect();
        self.worker_command_sender.send_replace(working_data);

        Ok(())
    }
}

#[async_trait]
impl super::BlockReceiver for FilesystemReceiver {
    async fn recv_block(&mut self) -> Result<Option<PrecomputedBlock>, anyhow::Error> {
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
                        if event
                            .tags
                            .iter()
                            .any(|signal|
                                matches!(signal, Tag::FileEventKind(Create(CreateKind::File)))
                                ||
                                matches!(signal, Tag::FileEventKind(Modify(_)))
                            )
                        {
                            let mut path_and_filetype = None;
                            for tag in event.tags.iter() {
                                match tag {
                                    watchexec::event::Tag::Path { path, file_type } => {
                                        path_and_filetype = Some((path, file_type))
                                    }
                                    _ => continue,
                                }
                            }

                            if let Some((path, Some(_filetype))) = path_and_filetype {
                                match parse_file(path.as_path()).await {
                                    Ok(block) => return Ok(Some(block)),
                                    Err(_) => continue,
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
