use anyhow::anyhow;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{debug, error, info, instrument, warn};

use crate::block::store::BlockStore;
use crate::block::{is_valid_state_hash, BlockHash, BlockWithoutHeight};
use crate::command::store::CommandStore;
use crate::command::Command;
use crate::ledger::store::LedgerStore;
use crate::ledger::{self, public_key};
use crate::state::summary::{SummaryShort, SummaryVerbose};
use crate::state::IndexerState;

/// Unix Domain Socket Server for the Mina Indexer
pub struct UnixDomainSocketServer {
    /// True if the Unix domain socket server is running
    is_running: AtomicBool,
    /// Unix domain socket file path
    path: PathBuf,
    /// The underlying data store
    state_ro: Arc<IndexerState>,
}

impl UnixDomainSocketServer {
    /// Create a new UnixDomainSocketServer
    pub fn new(path: PathBuf, state_ro: IndexerState) -> Self {
        Self {
            is_running: AtomicBool::new(false),
            state_ro: Arc::new(state_ro),
            path,
        }
    }
}

/// Start the Unix domain server
pub async fn start(server: UnixDomainSocketServer) -> anyhow::Result<()> {
    let result = server
        .is_running
        .compare_and_swap(false, true, Ordering::AcqRel);
    if result {
        warn!("Unix domain server already running");
        return Ok(());
    }

    // Remove unix domain socket file vestige if it exists
    let path = Path::new(&server.path);
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    let listener = UnixListener::bind(&server.path)?;
    info!("Unix domain socket server running on: {:?}", &server.path);

    tokio::spawn(run(server, listener));
    Ok(())
}

/// Stop the Unix domain server
pub async fn stop(server: &UnixDomainSocketServer) {
    server
        .is_running
        .compare_and_swap(true, false, Ordering::AcqRel);
    info!("Unix domain socket server stopped");
}

/// Accept client connections and spawn a green thread to handle the connection
async fn run(mut server: UnixDomainSocketServer, listener: UnixListener) -> anyhow::Result<()> {
    while *server.is_running.get_mut() {
        tokio::select! {
            client = listener.accept() => {
                match client {
                    Ok((socket, _)) => {
                        let state = server.state_ro.clone();
                        //state.sync_from_db();
                        tokio::spawn(async move {
                            let _ = handle_connection(socket, state).await;
                        });
                    }
                    Err(e) => error!("Failed to accept connection: {}", e),
                }
            }
        }
    }
    Ok(())
}

/// Handles the comminication/protocl between the uds client and server.
///
/// Protocol is undocumented but in short, it's basically as follows:
///
/// <letter> ::= a|b|c|d|e|f|g|h|i|j|k|l|m|n|o|p|q|r|s|t|u|v|w|x|y|z|
///              A|B|C|D|E|F|G|H|I|J|K|L|M|N|O|P|Q|R|S|T|U|V|W|X|Y|Z
///
/// <digit> ::= 0|1|2|3|4|5|6|7|8|9
///
/// <number> ::= <digit>
///            | <number> <digit>
///
/// <word> ::= <letter>
///          | <word> <letter>
///          | <word> <number>
///          | <word> '_'
///
/// <command> ::= <word>
///
/// <arg_list> ::= <word>
///         | <arg_list> <word>
///
/// <request_line> ::= <command> <arg_list>
///
#[instrument(skip_all)]
async fn handle_connection(socket: UnixStream, state: Arc<IndexerState>) -> anyhow::Result<()> {
    // Protocol rigmarole
    let (reader, mut writer) = socket.into_split();
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::with_capacity(1024);
    let read_size = reader.read_buf(&mut buffer).await?;

    let store = state.indexer_store.clone().unwrap();
    let database = &store.database;
    database.try_catch_up_with_primary()?;

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
            if public_key::is_valid(pk) {
                Some(pk.into())
            } else {
                Some(format!("Invalid pk: {pk}"))
            };

            info!("Received account command for {pk}");

            let ledger = state.ledger.clone();
            let account = ledger.accounts.get(&pk.into());
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
            let best_tip = state.best_tip.clone();
            let num = String::from_utf8(buffers.next().unwrap().to_vec())?.parse::<usize>()?;
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?.parse()?;
            let start_state_hash: BlockHash =
                String::from_utf8(buffers.next().unwrap().to_vec())?.into();
            let end_state_hash = {
                let hash = String::from_utf8(buffers.next().unwrap().to_vec())?;
                let hash = hash.trim_end_matches('\0');
                if !is_valid_state_hash(hash) {
                    best_tip.state_hash.clone()
                } else {
                    hash.into()
                }
            };
            // Deal with the unwrap in a cleaner way
            let db = state.indexer_store.clone().unwrap();
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
            let ledger = state.ledger.clone().to_string();
            match buffers.next() {
                Some(data_buffer) => {
                    let data = String::from_utf8(data_buffer.to_vec())?;
                    let data = data.trim_end_matches('\0');
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
                // TODO: Find a better way to unwrap
                let db = state.indexer_store.clone().unwrap();
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
            if is_valid_state_hash(&hash[..52]) {
                let hash = &hash[..52];
                debug!("{hash} is a state hash");
                // TODO: Find a better way to unwrap
                let db = state.indexer_store.clone().unwrap();
                if let Some(ledger) = db.get_ledger_state_hash(&hash.into())? {
                    let ledger = ledger.to_string();
                    match buffers.next() {
                        None => {
                            debug!("Writing ledger at state hash {hash} to stdout");
                            Some(ledger)
                        }
                        Some(path_buffer) => {
                            let path = &String::from_utf8(path_buffer.to_vec())?
                                .trim_end_matches('\0')
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
                // TODO: Find a better way to unwrap
                let db = state.indexer_store.clone().unwrap();
                if let Some(ledger) = db.get_ledger(hash)? {
                    let ledger = ledger.to_string();
                    match buffers.next() {
                        None => {
                            debug!("Writing ledger at {hash} to stdout");
                            Some(ledger)
                        }
                        Some(path_buffer) => {
                            let path = &String::from_utf8(path_buffer.to_vec())?
                                .trim_end_matches('\0')
                                .parse::<PathBuf>()?;
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
            let height = String::from_utf8(buffers.next().unwrap().to_vec())?
                .trim_end_matches('\0')
                .parse::<u32>()?;
            info!("Received ledger_at_height {height} command");
            let db = state.indexer_store.clone().unwrap();
            if let Some(ledger) = db.get_ledger_at_height(height)? {
                let ledger = ledger.to_string();
                match buffers.next() {
                    None => {
                        debug!("Writing ledger at height {height} to stdout");
                        Some(ledger)
                    }
                    Some(path_buffer) => {
                        let path = &String::from_utf8(path_buffer.to_vec())?
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
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?
                .trim_end_matches('\0')
                .parse::<bool>()?;
            let summary = &state.summary_verbose().clone();

            if verbose {
                Some(serde_json::to_string::<SummaryVerbose>(summary)?)
            } else {
                Some(serde_json::to_string::<SummaryShort>(
                    &summary.clone().into(),
                )?)
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
            let pk = &String::from_utf8(buffers.next().unwrap().to_vec())?;
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?.parse::<bool>()?;
            let num = String::from_utf8(buffers.next().unwrap().to_vec())?.parse::<usize>()?;
            let start_state_hash: BlockHash =
                String::from_utf8(buffers.next().unwrap().to_vec())?.into();
            let end_state_hash: BlockHash = {
                let raw = String::from_utf8(buffers.next().unwrap().to_vec())?;
                if &raw == "x" {
                    // dummy value
                    let best_tip = state.best_tip.clone();
                    best_tip.state_hash.clone()
                } else {
                    raw.into()
                }
            };
            let path = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let path = path.trim_end_matches('\0');

            info!("Received transactions command for public key {pk}");
            // TODO: Find a better way to unwrap
            let db = state.indexer_store.clone().unwrap();
            let transactions = db
                .get_commands_with_bounds(&pk.clone().into(), &start_state_hash, &end_state_hash)?
                .unwrap_or(vec![]);
            let transaction_str = {
                let txs = if num != 0 {
                    transactions.into_iter().take(num).collect()
                } else {
                    transactions
                };

                if verbose {
                    format!("{txs:?}")
                } else {
                    let txs: Vec<Command> =
                        txs.into_iter().map(|c| Command::from(c.command)).collect();
                    format!("{txs:?}")
                }
            };
            if path.is_empty() {
                debug!("Writing transactions for {pk} to stdout");
                Some(transaction_str)
            } else {
                let path: PathBuf = path.into();
                if !path.is_dir() {
                    debug!("Writing transactions for {pk} to {}", path.display());

                    tokio::fs::write(&path, transaction_str).await?;
                    Some(format!(
                        "Transactions for {pk} written to {}",
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
        "tx-hash" => {
            let tx_hash = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?
                .trim_end_matches('\0')
                .parse()?;

            info!("Received transactions command for tx hash {tx_hash}");
            let db = state.indexer_store.clone().unwrap();
            db.get_command_by_hash(&tx_hash)?.map(|cmd| {
                if verbose {
                    format!("{cmd:?}")
                } else {
                    let cmd: Command = cmd.command.into();
                    format!("{cmd:?}")
                }
            })
        }
        "tx-state-hash" => {
            let state_hash = String::from_utf8(buffers.next().unwrap().to_vec())?;
            let verbose = String::from_utf8(buffers.next().unwrap().to_vec())?
                .trim_end_matches('\0')
                .parse()?;

            info!("Received transactions command for state hash {state_hash}");
            let db = state.indexer_store.clone().unwrap();
            db.get_commands_in_block(&state_hash.into())?.map(|cmds| {
                if verbose {
                    format!("{cmds:?}")
                } else {
                    let cmd: Vec<Command> = cmds.into_iter().map(Command::from).collect();
                    format!("{cmd:?}")
                }
            })
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
