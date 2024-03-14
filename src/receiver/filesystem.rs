use super::{Parser, Receiver};
use crate::{
    block::{is_valid_block_file, parser::BlockParser, precomputed::PrecomputedBlock},
    ledger::staking::{is_valid_ledger_file, parser::StakingLedgerParser, StakingLedger},
};
use anyhow::bail;
use async_priority_channel as priority;
use async_trait::async_trait;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use tokio::sync::{
    mpsc,
    watch::{self, Sender},
};
use tracing::{debug, error, instrument};
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

pub struct FilesystemReceiver<Parser> {
    parsers: Vec<Parser>,
    paths_added: HashSet<PathBuf>,
    worker_command_sender: Sender<WorkingData>,
    worker_event_receiver: priority::Receiver<Event, Priority>,
    worker_error_receiver: mpsc::Receiver<RuntimeError>,
}

impl<P: Parser> FilesystemReceiver<P> {
    #[instrument]
    pub async fn new() -> anyhow::Result<Self> {
        debug!("Initializing new filesystem {} receiver", P::KIND);
        let (ev_s, worker_event_receiver) = priority::bounded(4096);
        let (er_s, worker_error_receiver) = mpsc::channel(128);
        let (worker_command_sender, wd_r) = watch::channel(WorkingData::default());

        tokio::spawn(async {
            debug!("spawning filesystem {} watcher worker", P::KIND);
            worker(wd_r, er_s, ev_s).await.expect("should not crash");
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
        self.parsers.iter().map(|parser| parser.path()).collect()
    }

    pub fn load_directory(&mut self, directory: impl AsRef<Path>) -> anyhow::Result<()> {
        debug!(
            "loading directory {} into filesystem {} receiver",
            directory.as_ref().display(),
            P::KIND,
        );

        if !directory.as_ref().is_dir() {
            bail!("{}", directory.as_ref().display());
        }

        let mut working_data = WorkingData::default();
        let mut watched_directories = self.watched_directories();

        debug!(
            "sending command to {} worker with new working data",
            P::KIND,
        );
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
impl Receiver<BlockParser> for FilesystemReceiver<BlockParser> {
    async fn recv_data(&mut self) -> anyhow::Result<Option<PrecomputedBlock>> {
        loop {
            tokio::select! {
                error_fut = self.worker_error_receiver.recv() => {
                    if let Some(error) = error_fut {
                        bail!("{} worker runtime error: {}", BlockParser::KIND, error)
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
                                    if is_valid_block_file(path) && !self.paths_added.contains(path) {
                                        self.paths_added.insert(path.clone());
                                        match PrecomputedBlock::parse_file(path.as_path()) {
                                            Ok(block) => return Ok(Some(block)),
                                            Err(e) => {
                                                error!("Cannot parse block at {}: {}", path.display(), e);
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
#[async_trait]
impl Receiver<StakingLedgerParser> for FilesystemReceiver<StakingLedgerParser> {
    async fn recv_data(&mut self) -> anyhow::Result<Option<StakingLedger>> {
        loop {
            tokio::select! {
                error_fut = self.worker_error_receiver.recv() => {
                    if let Some(error) = error_fut {
                        bail!("{} worker runtime error: {}", StakingLedgerParser::KIND, error);
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
                                    if is_valid_ledger_file(path) && !self.paths_added.contains(path) {
                                        self.paths_added.insert(path.clone());
                                        match StakingLedger::parse_file(path.as_path()) {
                                            Ok(staking_ledger) => return Ok(Some(staking_ledger)),
                                            Err(e) => {
                                                error!("Cannot parse staking ledger at {}: {}", path.display(), e);
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

impl Parser for BlockParser {
    type Data = PrecomputedBlock;

    const KIND: &'static str = "block";

    fn path(&self) -> PathBuf {
        self.blocks_dir.clone()
    }
}

impl Parser for StakingLedgerParser {
    type Data = StakingLedger;

    const KIND: &'static str = "ledger";

    fn path(&self) -> PathBuf {
        self.ledgers_dir.clone()
    }
}
