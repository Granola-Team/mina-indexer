use crate::{
    block::{self, store::BlockStore, Block, BlockHash, BlockWithoutHeight},
    command::{signed, store::CommandStore, Command},
    ledger::{self, public_key, store::LedgerStore, Ledger},
    server::{IndexerConfiguration, IpcChannelUpdate},
    snark_work::store::SnarkStore,
    state::summary::{SummaryShort, SummaryVerbose},
    store::IndexerStore,
};
use futures_util::{io::BufReader, AsyncBufReadExt, AsyncWriteExt};
use interprocess::local_socket::tokio::{LocalSocketListener, LocalSocketStream};
use std::{path::PathBuf, process, sync::Arc};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, instrument, trace, warn};

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
            ledger: RwLock::new(config.ledger.into()),
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
            let pk = String::from_utf8(pk_buffer.to_vec())?;
            let pk = pk.trim_end_matches('\0');
            info!("Received account command for {pk}");

            if let Some(ledger) = db.get_ledger_state_hash(&best_tip.state_hash)? {
                if !public_key::is_valid(pk) {
                    invalid_public_key(pk)
                } else if let Ok(pk) = pk.try_into() {
                    let account = ledger.accounts.get(&pk);
                    if let Some(account) = account {
                        info!("Writing account {pk} to client");
                        Some(format!("{account}"))
                    } else {
                        warn!("Account {pk} does not exist");
                        Some(format!("Account {pk} does not exist"))
                    }
                } else {
                    warn!("Invalid public key {pk}");
                    Some(format!("Invalid public key {pk}"))
                }
            } else {
                error!(
                    "Best ledger not in database (length {}): {}",
                    best_tip.blockchain_length, best_tip.state_hash.0
                );
                Some(format!(
                    "Best ledger not in database (length {}): {}",
                    best_tip.blockchain_length, best_tip.state_hash.0
                ))
            }
        }
        "best_chain" => {
            info!("Received best_chain command");
            let num = String::from_utf8(buffers.next().unwrap().to_vec())?.parse::<usize>()?;
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?.parse()?;
            let start_state_hash: BlockHash =
                String::from_utf8(buffers.next().unwrap().to_vec())?.into();
            let end_state_hash = {
                let hash = String::from_utf8(buffers.next().unwrap().to_vec())?;
                let hash = hash.trim_end_matches('\0');
                if !block::is_valid_state_hash(hash) {
                    best_tip.state_hash.clone()
                } else {
                    hash.into()
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
                    let data = String::from_utf8(data_buffer.to_vec())?;
                    let data = data.trim_end_matches('\0');
                    if data.is_empty() {
                        debug!("Writing best ledger to stdout");
                        Some(ledger)
                    } else {
                        let path = data.parse::<PathBuf>()?;
                        if !path.is_dir() {
                            debug!("Writing best ledger to {}", path.display());

                            tokio::fs::write(path.clone(), ledger).await?;
                            Some(format!("Best ledger written to {}", path.display()))
                        } else {
                            file_must_not_be_a_directory(&path)
                        }
                    }
                }
                _ => None,
            }
        }
        "checkpoint" => {
            info!("Received checkpoint command");
            let path: PathBuf = String::from_utf8(buffers.next().unwrap().to_vec())?
                .trim_end_matches('\0')
                .parse()?;
            if path.exists() {
                error!("Checkpoint directory already exists at {}", path.display());
                Some(format!(
                    "Checkpoint directory already exists at {}",
                    path.display()
                ))
            } else {
                debug!("Creating checkpoint at {}", path.display());
                db.create_checkpoint(&path)?;
                Some(format!(
                    "Checkpoint created and saved to {}",
                    path.display()
                ))
            }
        }
        "ledger" => {
            let hash = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let hash = hash.trim_end_matches('\0');
            info!("Received ledger command for {hash}");

            // check if ledger or state hash and use appropriate getter
            if block::is_valid_state_hash(&hash[..52]) {
                let hash = &hash[..52];
                trace!("{hash} is a state hash");

                if let Some(ledger) = db.get_ledger_state_hash(&hash.into())? {
                    let ledger = ledger.to_string();
                    match buffers.next() {
                        None => {
                            debug!("Writing ledger at state hash {hash} to stdout");
                            Some(ledger)
                        }
                        Some(path_buffer) => {
                            let path = String::from_utf8(path_buffer.to_vec())?
                                .trim_end_matches('\0')
                                .parse::<PathBuf>()?;
                            if !path.is_dir() {
                                debug!("Writing ledger at {hash} to {}", path.display());

                                tokio::fs::write(path.clone(), ledger).await?;
                                Some(format!(
                                    "Ledger at state hash {hash} written to {}",
                                    path.display()
                                ))
                            } else {
                                file_must_not_be_a_directory(&path)
                            }
                        }
                    }
                } else {
                    Some(format!("Ledger at state hash {hash} is not in the store"))
                }
            } else if ledger::is_valid_hash(&hash[..51]) {
                let hash = &hash[..51];
                trace!("{hash} is a ledger hash");

                if let Some(ledger) = db.get_ledger(hash)? {
                    let ledger = ledger.to_string();
                    match buffers.next() {
                        None => {
                            debug!("Writing ledger at {hash} to stdout");
                            Some(ledger)
                        }
                        Some(path_buffer) => {
                            let path = String::from_utf8(path_buffer.to_vec())?
                                .trim_end_matches('\0')
                                .parse::<PathBuf>()?;
                            if !path.is_dir() {
                                debug!("Writing ledger at {hash} to {}", path.display());

                                tokio::fs::write(path.clone(), ledger).await?;
                                Some(format!("Ledger at {hash} written to {}", path.display()))
                            } else {
                                file_must_not_be_a_directory(&path)
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
            let height = String::from_utf8(buffers.next().unwrap().to_vec())?
                .trim_end_matches('\0')
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
                        let path = String::from_utf8(path_buffer.to_vec())?
                            .trim_end_matches('\0')
                            .parse::<PathBuf>()?;
                        if !path.is_dir() {
                            debug!("Writing ledger at height {height} to {}", path.display());

                            tokio::fs::write(&path, ledger).await?;
                            Some(format!(
                                "Ledger at height {height} written to {}",
                                path.display()
                            ))
                        } else {
                            file_must_not_be_a_directory(&path)
                        }
                    }
                }
            } else {
                None
            }
        }
        "snark-pk" => {
            let pk = &String::from_utf8(buffers.next().unwrap().to_vec())?;
            let path = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let path = path.trim_end_matches('\0');
            info!("Received SNARK work command for public key {pk}");

            let snarks = db
                .get_snark_work_by_public_key(&pk.clone().into())?
                .unwrap_or(vec![]);
            let snarks_str = format!("{snarks:?}");

            if path.is_empty() {
                debug!("Writing SNARK work for {pk} to stdout");
                Some(snarks_str)
            } else {
                let path: PathBuf = path.into();
                if !path.is_dir() {
                    debug!("Writing SNARK work for {pk} to {}", path.display());

                    tokio::fs::write(&path, snarks_str).await?;
                    Some(format!("SNARK work for {pk} written to {}", path.display()))
                } else {
                    file_must_not_be_a_directory(&path)
                }
            }
        }
        "snark-state-hash" => {
            let state_hash = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let path = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let path = path.trim_end_matches('\0');
            info!("Received SNARK work command for state hash {state_hash}");

            db.get_snark_work_in_block(&state_hash.clone().into())?
                .and_then(|snarks| {
                    let snarks_str = format!("{snarks:?}");
                    if path.is_empty() {
                        debug!("Writing SNARK work for {state_hash} to stdout");
                        Some(snarks_str)
                    } else {
                        let path: PathBuf = path.into();
                        if !path.is_dir() {
                            debug!(
                                "Writing SNARK work for block {state_hash} to {}",
                                path.display()
                            );

                            std::fs::write(&path, snarks_str).unwrap();
                            Some(format!(
                                "SNARK work for block {state_hash} written to {}",
                                path.display()
                            ))
                        } else {
                            file_must_not_be_a_directory(&path)
                        }
                    }
                })
        }
        "summary" => {
            info!("Received summary command");
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?
                .trim_end_matches('\0')
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
            process::exit(0);
        }
        "tx-pk" => {
            let pk = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?.parse::<bool>()?;
            let start_state_hash: BlockHash =
                String::from_utf8(buffers.next().unwrap().to_vec())?.into();
            let end_state_hash: BlockHash = {
                let raw = String::from_utf8(buffers.next().unwrap().to_vec())?;
                if &raw == "x" {
                    // dummy value
                    best_tip.state_hash.clone()
                } else {
                    raw.into()
                }
            };
            info!("Received tx-public-key command for {pk}");

            if !public_key::is_valid(&pk) {
                invalid_public_key(&pk)
            } else if !block::is_valid_state_hash(&start_state_hash.0) {
                invalid_state_hash(&start_state_hash.0)
            } else if !block::is_valid_state_hash(&end_state_hash.0) {
                invalid_state_hash(&end_state_hash.0)
            } else {
                let path = String::from_utf8(buffers.next().unwrap().to_vec())?;
                let path = path.trim_end_matches('\0');
                let transactions = db
                    .get_commands_for_public_key(&pk.clone().into())?
                    .unwrap_or(vec![]);
                let transaction_str = if verbose {
                    format!("{transactions:#?}")
                } else {
                    let txs: Vec<Command> = transactions.into_iter().map(Command::from).collect();
                    format!("{txs:#?}")
                };

                if path.is_empty() {
                    debug!("Writing transactions for {pk} to stdout");
                    Some(transaction_str)
                } else {
                    let path: PathBuf = path.into();
                    if !path.is_dir() {
                        debug!("Writing transactions for {pk} to {}", path.display());

                        std::fs::write(&path, transaction_str)?;
                        Some(format!(
                            "Transactions for {pk} written to {}",
                            path.display()
                        ))
                    } else {
                        file_must_not_be_a_directory(&path)
                    }
                }
            }
        }
        "tx-hash" => {
            let tx_hash = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?
                .trim_end_matches('\0')
                .parse()?;

            info!("Received tx-hash command for {tx_hash}");
            if !signed::is_valid_tx_hash(&tx_hash) {
                invalid_tx_hash(&tx_hash)
            } else {
                db.get_command_by_hash(&tx_hash)?.map(|cmd| {
                    if verbose {
                        format!("{cmd:#?}")
                    } else {
                        let cmd: Command = cmd.command.into();
                        format!("{cmd:#?}")
                    }
                })
            }
        }
        "tx-state-hash" => {
            let state_hash = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?
                .trim_end_matches('\0')
                .parse()?;

            info!("Received tx-state-hash command for {state_hash}");
            if !block::is_valid_state_hash(&state_hash) {
                invalid_state_hash(&state_hash)
            } else {
                db.get_commands_in_block(&state_hash.into())?.map(|cmds| {
                    if verbose {
                        format!("{cmds:#?}")
                    } else {
                        let cmd: Vec<Command> = cmds.into_iter().map(Command::from).collect();
                        format!("{cmd:#?}")
                    }
                })
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

fn file_must_not_be_a_directory(path: &std::path::Path) -> Option<String> {
    Some(format!(
        "The path provided must not be a directory: {}",
        path.display()
    ))
}

fn invalid_public_key(input: &str) -> Option<String> {
    warn!("Invalid public key: {input}");
    Some(format!("Invalid public key: {input}"))
}

fn invalid_tx_hash(input: &str) -> Option<String> {
    warn!("Invalid transaction hash: {input}");
    Some(format!("Invalid transaction hash: {input}"))
}

fn invalid_state_hash(input: &str) -> Option<String> {
    warn!("Invalid state hash: {input}");
    Some(format!("Invalid state hash: {input}"))
}
