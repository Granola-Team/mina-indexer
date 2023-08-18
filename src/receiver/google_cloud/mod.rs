use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use async_ringbuf::{AsyncHeapConsumer, AsyncHeapRb};
use async_trait::async_trait;
use serde_derive::{Deserialize, Serialize};
use tokio::{
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::{debug, error, info, instrument, trace};

use crate::block::precomputed::PrecomputedBlock;

use self::worker::{
    GoogleCloudBlockWorker, GoogleCloudBlockWorkerCommand, GoogleCloudBlockWorkerData,
    GoogleCloudBlockWorkerError,
};

use super::BlockReceiver;

pub mod worker;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MinaNetwork {
    #[serde(rename = "mainnet")]
    Mainnet,
    #[serde(rename = "berkeley")]
    Berkeley,
    #[serde(rename = "testnet")]
    Testnet,
}

pub enum GoogleCloudBlockReceiverError {
    CommandError(mpsc::error::SendError<GoogleCloudBlockWorkerCommand>),
}

pub struct GoogleCloudBlockReceiver {
    blocks_consumer: AsyncHeapConsumer<PrecomputedBlock>,
    error_receiver: watch::Receiver<Option<GoogleCloudBlockWorkerError>>,
    command_sender: mpsc::Sender<GoogleCloudBlockWorkerCommand>,
    worker_data_receiver: watch::Receiver<GoogleCloudBlockWorkerData>,
    worker_join_handle: Option<JoinHandle<()>>,
}

impl GoogleCloudBlockReceiver {
    #[instrument]
    pub async fn new(
        max_length: u64,
        lookup_num: u64,
        temp_blocks_dir: impl AsRef<Path> + std::fmt::Debug,
        update_freq: Duration,
        network: MinaNetwork,
        bucket: String,
    ) -> Result<Self, anyhow::Error> {
        info!("initializing new GoogleCloudBlockReceiver");
        let temp_blocks_dir = PathBuf::from(temp_blocks_dir.as_ref());
        let (blocks_producer, blocks_consumer) =
            AsyncHeapRb::new((lookup_num) as usize).split();
        let (error_sender, error_receiver) = watch::channel(None);
        let (command_sender, command_receiver) = mpsc::channel(1);

        let worker_data = GoogleCloudBlockWorkerData {
            max_length,
            lookup_num,
            temp_blocks_dir,
            update_freq,
            network,
            bucket,
        };

        let (worker_data_sender, worker_data_receiver) = watch::channel(worker_data.clone());

        let mut worker = GoogleCloudBlockWorker::new(
            worker_data,
            blocks_producer,
            error_sender,
            command_receiver,
            worker_data_sender,
        )?;

        debug!("spawning new GoogleCloudBlockWorker");
        let worker_join_handle = tokio::spawn(async move {
            worker.start_loop().await;
        });

        Ok(Self {
            blocks_consumer,
            error_receiver,
            command_sender,
            worker_data_receiver,
            worker_join_handle: Some(worker_join_handle),
        })
    }

    #[instrument(skip(self))]
    pub async fn set_worker_data(
        &self,
        worker_data: GoogleCloudBlockWorkerData,
    ) -> Result<(), GoogleCloudBlockReceiverError> {
        trace!("updating worker data for GoogleCloudBlockWorker");
        self.command_sender
            .send(GoogleCloudBlockWorkerCommand::SetWorkerData(worker_data))
            .await
            .map_err(GoogleCloudBlockReceiverError::CommandError)
    }

    #[instrument(skip(self))]
    pub async fn get_worker_data(
        &self,
    ) -> Result<GoogleCloudBlockWorkerData, GoogleCloudBlockReceiverError> {
        trace!("getting worker data from GoogleCloudBlockWorker");
        self.command_sender
            .send(GoogleCloudBlockWorkerCommand::GetWorkerData)
            .await
            .map_err(GoogleCloudBlockReceiverError::CommandError)?;
        Ok(self.worker_data_receiver.borrow().clone())
    }
}

#[async_trait]
impl BlockReceiver for GoogleCloudBlockReceiver {
    #[instrument(skip(self))]
    async fn recv_block(&mut self) -> Result<Option<PrecomputedBlock>, anyhow::Error> {
        tokio::select! {
            block = self.blocks_consumer.pop() => {
                info!("received block from GoogleCloudBlockWorker");
                return Ok(block);
            },
            error = self.error_receiver.changed() => {
                match error {
                    Ok(_) => {
                        let error = self.error_receiver.borrow().clone();
                        error!("GoogleCloudBlockWorker errored: {:?}", error);
                        return Err(error.expect("error channel only changes when an error is present").into());
                    },
                    Err(receiver_error) => return Err(receiver_error.into()),
                }
            }
        }
    }
}

impl Drop for GoogleCloudBlockReceiver {
    fn drop(&mut self) {
        info!("shutting down GoogleCloudBlockWorker");
        let command_sender = self.command_sender.clone();
        let worker_join_handle = self.worker_join_handle.take();
        let temp_block_dir = self.worker_data_receiver.borrow().temp_blocks_dir.clone();
        tokio::spawn(async move {
            command_sender
                .clone()
                .send(GoogleCloudBlockWorkerCommand::Shutdown).await
                .expect("shutdown command sends correctly");

            if let Some(join_handle) = worker_join_handle {
                join_handle.await.expect("worker fininshes successfully");
            }
        });
        debug!(
            "removing temporary block directory at {}",
            temp_block_dir.display()
        );
        std::fs::remove_dir_all(temp_block_dir).expect("block dir exists");
    }
}

impl std::fmt::Display for MinaNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MinaNetwork::Mainnet => "mainnet",
            MinaNetwork::Berkeley => "berkeley",
            MinaNetwork::Testnet => "testnet",
        })
    }
}

pub fn bucket_file_from_length(network: MinaNetwork, bucket: &str, length: u64) -> String {
    format!("gs://{bucket}/{}-{length}-*.json\n", network)
}
