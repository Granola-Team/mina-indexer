use crate::block::{parse_file, parser::BlockParser, precomputed::PrecomputedBlock};
use async_priority_channel as priority;
use std::path::{Path, PathBuf};
use tokio::sync::{
    mpsc,
    watch::{self, Sender},
};
use tracing::debug;
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

pub struct BlockReceiver {
    parsers: Vec<BlockParser>,
    worker_command_sender: Sender<WorkingData>,
    worker_event_receiver: priority::Receiver<Event, Priority>,
    worker_error_receiver: mpsc::Receiver<RuntimeError>,
}

pub struct ReceivedBlock {
    pub blocks_directory: PathBuf,
    pub block: PrecomputedBlock,
}

impl BlockReceiver {
    pub async fn new() -> anyhow::Result<BlockReceiver> {
        debug!("Building new block receiver");

        let (ev_s, worker_event_receiver) = priority::bounded(1024);
        let (er_s, worker_error_receiver) = mpsc::channel(64);
        let (worker_command_sender, wd_r) = watch::channel(WorkingData::default());

        tokio::spawn(async {
            debug!("Spawn worker");
            worker(wd_r, er_s, ev_s).await.unwrap();
        });

        let parsers = Vec::new();
        Ok(BlockReceiver {
            parsers,
            worker_command_sender,
            worker_event_receiver,
            worker_error_receiver,
        })
    }

    pub async fn load_directory(&mut self, directory: &Path) -> anyhow::Result<()> {
        debug!("Loading directory");

        if !directory.is_dir() {
            return Err(anyhow::Error::msg(format!(
                "
[BlockReceiver::load_directory]
    Adding directory {:} to the watched directories
    {:} is not a directory!",
                directory.display(),
                directory.display()
            )));
        }

        let mut wkd = WorkingData::default();
        wkd.pathset = vec![directory.into()];
        self.worker_command_sender.send_replace(wkd);

        match BlockParser::new(directory) {
            Ok(block_parser) => self.parsers.push(block_parser),
            Err(err) => return Err(err),
        }
        Ok(())
    }

    pub async fn recv(&mut self) -> Option<anyhow::Result<PrecomputedBlock>> {
        loop {
            tokio::select! {
                error_fut = self.worker_error_receiver.recv() => {
                    if let Some(error) = error_fut {
                        return Some(Err(error.into()));
                    }
                    return None;
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
                                    Ok(block) => return Some(Ok(block)),
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
