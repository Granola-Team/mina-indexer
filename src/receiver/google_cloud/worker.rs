use async_ringbuf::AsyncHeapProducer;
use serde_derive::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::{
    fs::read_dir,
    io::AsyncWriteExt,
    process::Command,
    sync::{mpsc, watch},
    time::sleep,
};
use tracing::{debug, error, instrument, trace};

use crate::block::{is_valid_block_file, parse_file, precomputed::PrecomputedBlock};

use super::{bucket_file_from_length, MinaNetwork};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleCloudBlockWorkerData {
    pub max_length: u64,
    pub overlap_num: u64,
    pub temp_blocks_dir: PathBuf,
    pub update_freq: Duration,
    pub network: MinaNetwork,
    pub bucket: String,
}

#[derive(Debug, Error, Clone)]
pub enum GoogleCloudBlockWorkerError {
    TempBlocksDirIsNotADirectory(PathBuf),
    IOError(String),
    BlockParseError(PathBuf, String),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoogleCloudBlockWorkerCommand {
    Shutdown,
    GetWorkerData,
    SetWorkerData(GoogleCloudBlockWorkerData),
}

pub struct GoogleCloudBlockWorker {
    worker_data: GoogleCloudBlockWorkerData,
    blocks_sender: AsyncHeapProducer<PrecomputedBlock>,
    error_sender: watch::Sender<Option<GoogleCloudBlockWorkerError>>,
    command_receiver: mpsc::Receiver<GoogleCloudBlockWorkerCommand>,
    worker_data_sender: watch::Sender<GoogleCloudBlockWorkerData>,
}

impl GoogleCloudBlockWorker {
    pub fn new(
        worker_data: GoogleCloudBlockWorkerData,
        blocks_sender: AsyncHeapProducer<PrecomputedBlock>,
        error_sender: watch::Sender<Option<GoogleCloudBlockWorkerError>>,
        command_receiver: mpsc::Receiver<GoogleCloudBlockWorkerCommand>,
        worker_data_sender: watch::Sender<GoogleCloudBlockWorkerData>,
    ) -> Result<Self, GoogleCloudBlockWorkerError> {
        if !worker_data.temp_blocks_dir.is_dir() {
            return Err(GoogleCloudBlockWorkerError::TempBlocksDirIsNotADirectory(
                worker_data.temp_blocks_dir,
            ));
        }
        Ok(Self {
            worker_data,
            blocks_sender,
            error_sender,
            command_receiver,
            worker_data_sender,
        })
    }

    #[instrument(skip(self))]
    pub async fn start_loop(&mut self) -> () {
        loop {
            debug!("starting new GoogleCloudBlockWorker work unit");
            let work_unit_started = Instant::now();

            if let Ok(command) = self.command_receiver.try_recv() {
                match command {
                    GoogleCloudBlockWorkerCommand::Shutdown => {
                        trace!("shutting down GoogleCloudBlockWorker");
                        return;
                    }
                    GoogleCloudBlockWorkerCommand::GetWorkerData => {
                        trace!("sending GoogleCloudBlockWorkerData to BlockReceiver");
                        self.worker_data_sender
                            .send_replace(self.worker_data.clone());
                    }
                    GoogleCloudBlockWorkerCommand::SetWorkerData(new_worker_data) => {
                        trace!("changing worker data for GoogleCloudBlockWorker");
                        self.worker_data = new_worker_data;
                    }
                }
            }

            trace!("downloading blocks within radius of {} from the best tip at length {} from google cloud bucket {} on network {}"
                , self.worker_data.overlap_num, self.worker_data.max_length, self.worker_data.bucket, self.worker_data.network
            );
            if let Err(worker_error) = gsutil_download_blocks(
                &self.worker_data.temp_blocks_dir,
                self.worker_data.max_length,
                self.worker_data.overlap_num,
                &self.worker_data.bucket,
                self.worker_data.network,
            )
            .await
            {
                error!("GoogleCloudBlockWorker error: {}", worker_error);
                self.error_sender.send_replace(Some(worker_error));
            }

            trace!(
                "parsing downloaded blocks from temporary directory {}",
                self.worker_data.temp_blocks_dir.display()
            );
            match parse_temp_blocks_dir(
                &mut self.worker_data.max_length,
                &self.worker_data.temp_blocks_dir,
            )
            .await
            {
                Err(e) => {
                    error!(
                        "parsing error for latest GoogleCloudBlockWorker download: {}",
                        e
                    );
                    self.error_sender.send_replace(Some(e));
                }
                Ok(precomputed_blocks) => {
                    debug!(
                        "parsed {} blocks successfully from {}, pushing to receiver",
                        precomputed_blocks.len(),
                        self.worker_data.temp_blocks_dir.display()
                    );
                    self.blocks_sender
                        .push_iter(precomputed_blocks.into_iter())
                        .await
                        .expect("consumer will not be dropped as long as worker is active")
                }
            }

            debug!("GoogleCloudBlockWorker finished work unit, waiting until next cycle");
            let work_unit_finished = Instant::now();
            let work_unit_duration = work_unit_finished.duration_since(work_unit_started);
            if work_unit_duration < self.worker_data.update_freq {
                sleep(self.worker_data.update_freq - work_unit_duration).await;
            }
        }
    }
}

#[instrument]
async fn gsutil_download_blocks(
    temp_blocks_dir: impl AsRef<Path> + std::fmt::Debug,
    max_height: u64,
    overlap_num: u64,
    blocks_bucket: impl AsRef<str> + std::fmt::Debug,
    network: MinaNetwork,
) -> Result<(), GoogleCloudBlockWorkerError> {
    trace!("spawning child gsutil process");
    let mut child = Command::new("gsutil")
        .stdin(Stdio::piped())
        .arg("-m")
        .arg("cp")
        .arg("-n")
        .arg("-I")
        .arg(AsRef::<Path>::as_ref(temp_blocks_dir.as_ref()))
        .spawn()
        .map_err(|e| GoogleCloudBlockWorkerError::IOError(e.to_string()))?;
    let mut child_stdin = child.stdin.take().unwrap();

    let start = 2.max(max_height.saturating_sub(overlap_num));
    let end = max_height + overlap_num;

    for length in start..=end {
        let bucket_file = bucket_file_from_length(network, blocks_bucket.as_ref(), length);
        trace!("downloading bucket file {}", bucket_file);
        child_stdin
            .write_all(bucket_file.as_bytes())
            .await
            .map_err(|e| GoogleCloudBlockWorkerError::IOError(e.to_string()))?;
    }

    Ok(())
}

#[instrument]
async fn parse_temp_blocks_dir(
    max_length: &mut u64,
    temp_blocks_dir: impl AsRef<Path> + std::fmt::Debug,
) -> Result<Vec<PrecomputedBlock>, GoogleCloudBlockWorkerError> {
    trace!(
        "opening temporary blocks directory at {}",
        temp_blocks_dir.as_ref().display()
    );
    let mut precomputed_blocks = vec![];
    let mut temp_dir_entries = read_dir(temp_blocks_dir).await.map_err(|read_dir_error| {
        GoogleCloudBlockWorkerError::IOError(read_dir_error.to_string())
    })?;
    while let Some(entry) = temp_dir_entries
        .next_entry()
        .await
        .map_err(|next_entry_error| {
            GoogleCloudBlockWorkerError::IOError(next_entry_error.to_string())
        })?
    {
        trace!(
            "parsing potential PrecomputedBlock from file at {}",
            &entry.path().display()
        );
        if !is_valid_block_file(&entry.path()) {
            continue;
        }

        let precomputed_block = parse_file(&entry.path()).await.map_err(|parse_error| {
            GoogleCloudBlockWorkerError::BlockParseError(entry.path(), parse_error.to_string())
        })?;

        trace!(
            "PrecomputedBlock parsed with state hash {}",
            &precomputed_block.state_hash
        );
        if let Some(length) = precomputed_block.blockchain_length {
            if length as u64 > *max_length {
                trace!("new max blockchain length found");
                *max_length = length as u64;
            }
        }
        precomputed_blocks.push(precomputed_block);

        if entry.metadata().await.is_ok() {
            trace!(
                "removing temporary block file from {}",
                &entry.path().display()
            );
            tokio::fs::remove_file(entry.path())
                .await
                .expect("file guaranteed to exist");
        }
    }

    Ok(precomputed_blocks)
}

impl std::fmt::Display for GoogleCloudBlockWorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoogleCloudBlockWorkerError::TempBlocksDirIsNotADirectory(not_directory) => f
                .write_str(&format!(
                    "temporary block directory {} is not a directory",
                    not_directory.display()
                )),
            GoogleCloudBlockWorkerError::IOError(io_error) => {
                f.write_str(&format!("encountered an IOError: {}", io_error))
            }
            GoogleCloudBlockWorkerError::BlockParseError(block_file, parse_error) => {
                f.write_str(&format!(
                    "could not parse block file {}: {}",
                    block_file.display(),
                    parse_error
                ))
            }
        }
    }
}
