use crate::{
    block::{is_valid_state_hash, store::BlockStore, Block, BlockHash, BlockWithoutHeight},
    command::{store::CommandStore, Command},
    ledger::{self, public_key::PublicKey, store::LedgerStore, Ledger},
    server::{IndexerConfiguration, IpcChannelUpdate},
    state::summary::{SummaryShort, SummaryVerbose},
    store::IndexerStore,
};
use futures_util::{io::BufReader, AsyncBufReadExt, AsyncWriteExt};
use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};
use std::{path::PathBuf, process, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, instrument, warn};

#[derive(Debug)]
pub struct IpcActor {
    state_recv: IpcStateReceiver,
    listener: LocalSocketListener,
    best_tip: RwLock<Block>,
    ledger: RwLock<Ledger>,
    summary: RwLock<Option<SummaryVerbose>>,
    store: RwLock<Arc<IndexerStore>>,
}
type IpcStateReceiver = mpsc::Receiver<IpcChannelUpdate>;
#[derive(Debug, Hash, PartialEq, Eq)]
pub enum IpcActorError {}

impl IpcActor {
    #[instrument(skip_all)]
    pub fn new(
        config: IndexerConfiguration,
        listener: LocalSocketListener,
        store: Arc<IndexerStore>,
        state_recv: IpcStateReceiver,
    ) -> Self {
        debug!("Creating new IPC actor");
        Self {
            state_recv,
            listener,
            best_tip: RwLock::new(Block {
                parent_hash: config.root_hash.clone(),
                state_hash: config.root_hash,
                height: 1,
                blockchain_length: 1,
                global_slot_since_genesis: 0,
            }),
            ledger: RwLock::new(config.ledger.ledger.into()),
            summary: RwLock::new(None),
            store: RwLock::new(store),
        }
    }

    #[instrument(skip(self))]
    pub async fn run(&mut self) -> () {
        loop {
            tokio::select! {
                state = self.state_recv.recv() => {
                    debug!("Received IPC state update");
                    match state {
                        None => panic!("IPC channel closed"),
                        Some(state) => {
                            debug!("Setting IPC state");
                            *self.best_tip.write().await = state.best_tip;
                            *self.ledger.write().await = state.ledger;
                            *self.summary.write().await = Some(*state.summary);
                            *self.store.write().await = state.store;
                        },
                    }
                }

                client = self.listener.accept() => {
                    let store = self.store.read().await.clone();
                    let best_tip = self.best_tip.read().await.clone();
                    let ledger = self.ledger.read().await.clone();
                    let summary = self.summary.read().await.clone();
                    match client {
                        Err(e) => error!("Error accepting connection: {}", e.to_string()),
                        Ok(stream) => {
                            info!("Accepted client connection");
                            tokio::spawn(async move {
                                debug!("Handling client connection");
                                match handle_conn(stream,
                                    &store,
                                    &best_tip,
                                    &ledger,
                                    summary.as_ref()
                                ).await {
                                    Err(e) => {
                                        error!("Error handling connection: {e}");
                                    },
                                    Ok(_) => { info!("handled connection"); },
                                };
                                debug!("Removing readonly instance at {}", store.db_path.clone().display());
                                tokio::fs::remove_dir_all(&store.db_path).await.ok();
                            });
                        }
                    }
                }
            }
        }
    }
}

#[instrument(skip_all)]
async fn handle_conn(
    conn: LocalSocketStream,
    db: &IndexerStore,
    best_tip: &Block,
    ledger: &Ledger,
    summary: Option<&SummaryVerbose>,
) -> Result<(), anyhow::Error> {
    use anyhow::anyhow;
    let (reader, mut writer) = conn.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(1024);
    let read_size = reader.read_until(0, &mut buffer).await?;

    if read_size == 0 {
        return Err(anyhow!("Unexpected EOF"));
    }

    let mut buffers = buffer.split(|byte| *byte == b' ');
    let command = buffers.next().unwrap();
    let command_string = String::from_utf8(command.to_vec()).unwrap();
    let response_json = match command_string.as_str() {
        "account" => {
            let pk_buffer = buffers.next().unwrap();
            let pk = PublicKey::from_address(&String::from_utf8(
                pk_buffer[..pk_buffer.len() - 1].to_vec(),
            )?)?;
            info!("Received account command for {pk}");

            let account = ledger.accounts.get(&pk);
            if let Some(account) = account {
                debug!("Writing account {account:?} to client");
                Some(serde_json::to_string(account)?)
            } else {
                warn!("Account {} does not exist", pk);
                Some(format!("Account {} does not exist", pk))
            }
        }
        "best_chain" => {
            info!("Received best_chain command");
            let num = String::from_utf8(buffers.next().unwrap().to_vec())?.parse::<usize>()?;
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?.parse()?;
            let start_state_hash: BlockHash =
                String::from_utf8(buffers.next().unwrap().to_vec())?.into();
            let end_state_hash = {
                let x = String::from_utf8(buffers.next().unwrap().to_vec())?;
                let x = x.trim_matches('\0');
                if !is_valid_state_hash(x) {
                    best_tip.state_hash.clone()
                } else {
                    x.into()
                }
            };

            let block = db.get_block(&end_state_hash)?.unwrap();
            let mut parent_hash = block.previous_state_hash();
            let mut best_chain = vec![block];
            if num == 0 {
                // no num bound => use hash bounds
                while let Some(parent_pcb) = db.get_block(&parent_hash)? {
                    let curr_hash: BlockHash = parent_pcb.state_hash.clone().into();
                    parent_hash = parent_pcb.previous_state_hash();
                    best_chain.push(parent_pcb);

                    if curr_hash == start_state_hash {
                        break;
                    }
                }
            } else {
                // num bound
                for _ in 1..num {
                    if let Some(parent_pcb) = db.get_block(&parent_hash)? {
                        let curr_hash: BlockHash = parent_pcb.state_hash.clone().into();
                        parent_hash = parent_pcb.previous_state_hash();
                        best_chain.push(parent_pcb);

                        if curr_hash == start_state_hash {
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }

            if verbose {
                Some(serde_json::to_string(&best_chain)?)
            } else {
                let best_chain: Vec<BlockWithoutHeight> =
                    best_chain.iter().map(BlockWithoutHeight::from).collect();
                Some(format!("{best_chain:?}"))
            }
        }
        "best_ledger" => {
            info!("Received best_ledger command");
            let ledger = ledger.to_string();
            match buffers.next() {
                Some(data_buffer) => {
                    let data = String::from_utf8(data_buffer[..data_buffer.len() - 1].to_vec())?;
                    if data.is_empty() {
                        debug!("Writing best ledger to stdout");
                        Some(ledger)
                    } else {
                        let path = &data.parse::<PathBuf>()?;
                        if !path.is_dir() {
                            debug!("Writing best ledger to {}", path.display());

                            tokio::fs::write(path, ledger).await?;
                            Some(format!("Best ledger written to {}", path.display()))
                        } else {
                            Some(format!(
                                "The path provided must not be a directory: {}",
                                path.display()
                            ))
                        }
                    }
                }
                _ => None,
            }
        }
        "ledger" => {
            let hash_buffer = buffers.next().unwrap();
            let hash = String::from_utf8(hash_buffer.to_vec())?;
            info!("Received ledger command for {hash}");

            // check if ledger or state hash and use appropriate getter
            if is_valid_state_hash(&hash[..52]) {
                let hash = &hash[..52];
                debug!("{hash} is a state hash");

                if let Some(ledger) = db.get_ledger_state_hash(&hash.into())? {
                    let ledger = ledger.to_string();
                    match buffers.next() {
                        None => {
                            debug!("Writing ledger at state hash {hash} to stdout");
                            Some(ledger)
                        }
                        Some(path_buffer) => {
                            let path =
                                &String::from_utf8(path_buffer[..path_buffer.len() - 1].to_vec())?
                                    .parse::<PathBuf>()?;
                            if !path.is_dir() {
                                debug!("Writing ledger at {hash} to {}", path.display());

                                tokio::fs::write(path, ledger).await?;
                                Some(format!(
                                    "Ledger at state hash {hash} written to {}",
                                    path.display()
                                ))
                            } else {
                                Some(format!(
                                    "The path provided must not be a directory: {}",
                                    path.display()
                                ))
                            }
                        }
                    }
                } else {
                    Some(format!("Ledger at state hash {hash} is not in the store"))
                }
            } else if ledger::is_valid_hash(&hash[..51]) {
                let hash = &hash[..51];
                debug!("{hash} is a ledger hash");

                if let Some(ledger) = db.get_ledger(hash)? {
                    let ledger = ledger.to_string();
                    match buffers.next() {
                        None => {
                            debug!("Writing ledger at {hash} to stdout");
                            Some(ledger)
                        }
                        Some(path_buffer) => {
                            let path =
                                &String::from_utf8(path_buffer.to_vec())?.parse::<PathBuf>()?;
                            if !path.is_dir() {
                                debug!("Writing ledger at {hash} to {}", path.display());

                                tokio::fs::write(path, ledger).await?;
                                Some(format!("Ledger at {hash} written to {}", path.display()))
                            } else {
                                Some(format!(
                                    "The path provided must not be a directory: {}",
                                    path.display()
                                ))
                            }
                        }
                    }
                } else {
                    Some(format!("Ledger at {hash} is not in the store"))
                }
            } else {
                debug!("Length 52: {}", hash.len());
                Some(format!("Invalid: {hash} is not a state or ledger hash"))
            }
        }
        "ledger_at_height" => {
            let height_buffer = buffers.next().unwrap();
            let height = String::from_utf8(height_buffer[..height_buffer.len() - 1].to_vec())?
                .parse::<u32>()?;
            info!("Received ledger_at_height {height} command");

            if height > best_tip.blockchain_length {
                Some(format!("Invalid query: ledger at height {height} cannot be determined from a best chain of length {}", best_tip.blockchain_length))
            } else if let Some(ledger) = db.get_ledger_at_height(height)? {
                let ledger = ledger.to_string();
                match buffers.next() {
                    None => {
                        debug!("Writing ledger at height {height} to stdout");
                        Some(ledger)
                    }
                    Some(path_buffer) => {
                        let path =
                            &String::from_utf8(path_buffer[..path_buffer.len() - 1].to_vec())?
                                .parse::<PathBuf>()?;
                        if !path.is_dir() {
                            debug!("Writing ledger at height {height} to {}", path.display());

                            tokio::fs::write(&path, ledger).await?;
                            Some(format!(
                                "Ledger at height {height} written to {}",
                                path.display()
                            ))
                        } else {
                            Some(format!(
                                "The path provided must not be a directory: {}",
                                path.display()
                            ))
                        }
                    }
                }
            } else {
                None
            }
        }
        "summary" => {
            info!("Received summary command");
            let data_buffer = buffers.next().unwrap();
            let verbose = String::from_utf8(data_buffer[..data_buffer.len() - 1].to_vec())?
                .parse::<bool>()?;
            if let Some(summary) = summary {
                if verbose {
                    Some(serde_json::to_string::<SummaryVerbose>(summary)?)
                } else {
                    Some(serde_json::to_string::<SummaryShort>(
                        &summary.clone().into(),
                    )?)
                }
            } else {
                Some("No summary available yet".into())
            }
        }
        "shutdown" => {
            info!("Received shutdown command");
            writer
                .write_all(b"Shutting down the Mina Indexer daemon...")
                .await?;
            info!("Shutting down the indexer...");
            process::exit(0);
        }
        "transactions" => {
            let pk_buffer = buffers.next().unwrap();
            let pk = String::from_utf8(pk_buffer.to_vec())?;
            let verbose_buffer = buffers.next().unwrap();
            let verbose = String::from_utf8(verbose_buffer[..verbose_buffer.len() - 1].to_vec())?
                .parse::<bool>()?;

            info!("Received transactions command for {pk}");
            let transactions = db
                .get_commands_for_public_key(&pk.clone().into())?
                .unwrap_or(vec![]);
            let transactions = {
                if verbose {
                    serde_json::to_string(&transactions)?
                } else {
                    let transactions_summary: Vec<Command> =
                        transactions.into_iter().map(Command::from).collect();
                    serde_json::to_string(&transactions_summary)?
                }
            };
            match buffers.next() {
                None => {
                    debug!("Writing transactions for {pk} to stdout");
                    Some(transactions)
                }
                Some(path_buffer) => {
                    let path = &String::from_utf8(path_buffer[..path_buffer.len() - 1].to_vec())?
                        .parse::<PathBuf>()?;
                    if !path.is_dir() {
                        debug!("Writing transactions for {pk} to {}", path.display());
                        tokio::fs::write(&path, transactions).await?;
                        Some(serde_json::to_string(&format!(
                            "Transactions for {pk} written to {}",
                            path.display()
                        ))?)
                    } else {
                        Some(serde_json::to_string(&format!(
                            "The path provided must not be a directory: {}",
                            path.display()
                        ))?)
                    }
                }
            }
        }
        bad_request => {
            return Err(anyhow!("Malformed request: {bad_request}"));
        }
    };

    if let Some(response_json) = response_json {
        writer.write_all(response_json.as_bytes()).await?;
    } else {
        writer
            .write_all(serde_json::to_string("no response 404")?.as_bytes())
            .await?;
    }

    Ok(())
}
