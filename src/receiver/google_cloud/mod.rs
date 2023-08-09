use std::{path::PathBuf, time::Duration};

use async_ringbuf::{AsyncHeapConsumer, AsyncHeapRb};
use async_trait::async_trait;
use serde_derive::{Serialize, Deserialize};
use tokio::sync::{mpsc, watch};

use crate::block::precomputed::PrecomputedBlock;

use self::worker::{GoogleCloudBlockWorkerError, GoogleCloudBlockWorkerCommand, GoogleCloudBlockWorker, GoogleCloudBlockWorkerData};

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
    CommandError(mpsc::error::SendError<GoogleCloudBlockWorkerCommand>)
}

pub struct GoogleCloudBlockReceiver {
    blocks_consumer: AsyncHeapConsumer<PrecomputedBlock>,
    error_receiver: watch::Receiver<Option<GoogleCloudBlockWorkerError>>,
    command_sender: mpsc::Sender<GoogleCloudBlockWorkerCommand>,
    worker_data_receiver: watch::Receiver<GoogleCloudBlockWorkerData>,
}

impl GoogleCloudBlockReceiver {
    pub async fn new(
        max_length: u64,
        overlap_num: u64,
        temp_blocks_dir: PathBuf,
        update_freq: Duration,
        network: MinaNetwork,
        bucket: String,
    ) -> Result<Self, anyhow::Error> {
        let (blocks_producer, blocks_consumer) = 
            AsyncHeapRb::new((overlap_num * 2) as usize).split();
        let (error_sender, error_receiver) = 
            watch::channel(None);
        let (command_sender, command_receiver) =
            mpsc::channel(1);

        let worker_data = GoogleCloudBlockWorkerData {
            max_length,
            overlap_num,
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
            worker_data_sender
        )?;

        tokio::spawn(async move { 
            worker.start_loop().await;
        });

        Ok(Self {
            blocks_consumer,
            error_receiver,
            command_sender,
            worker_data_receiver,
        })
    }

    pub async fn set_worker_data(&self, worker_data: GoogleCloudBlockWorkerData) -> Result<(), GoogleCloudBlockReceiverError> {
        self.command_sender.send(
            GoogleCloudBlockWorkerCommand::SetWorkerData(worker_data)
        ).await.map_err(|send_error| GoogleCloudBlockReceiverError::CommandError(send_error))
    }

    pub async fn get_worker_data(&self) -> Result<GoogleCloudBlockWorkerData, GoogleCloudBlockReceiverError> {
        self.command_sender.send(
            GoogleCloudBlockWorkerCommand::GetWorkerData
        ).await.map_err(|send_error| GoogleCloudBlockReceiverError::CommandError(send_error))?;
        Ok(self.worker_data_receiver.borrow().clone())
    }
}

#[async_trait]
impl BlockReceiver for GoogleCloudBlockReceiver {
    async fn recv_block(&mut self) -> Result<Option<PrecomputedBlock>, anyhow::Error> {
        tokio::select! {
            block = self.blocks_consumer.pop() => {
                return Ok(block);
            },
            error = self.error_receiver.changed() => {
                match error {
                    Ok(_) => {
                        let error = self.error_receiver.borrow().clone();
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
        self.command_sender.blocking_send(GoogleCloudBlockWorkerCommand::Shutdown)
            .expect("shutdown command sends correctly");
    }
}

impl MinaNetwork {
    pub fn to_string(&self) -> String {
        String::from(match self {
            MinaNetwork::Mainnet => "mainnet",
            MinaNetwork::Berkeley => "berkeley",
            MinaNetwork::Testnet => "testnet",
        })
    }
}

pub fn bucket_file_from_length(network: MinaNetwork, bucket: &str, length: u64) -> String {
    format!("gs://{bucket}/{}-{length}-*.json\n", network.to_string())
}