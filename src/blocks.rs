use std::path::Path;
use serde_json::Value;
use tokio::fs::{File, ReadDir};
use tokio::io::AsyncReadExt;

pub struct BlockLog {
    pub log_name: String,
    json: Value
}

#[derive(Debug)]
pub enum LogsError {
    IOError(std::io::Error),
    JSONError(serde_json::Error)
}

pub struct LogsProcessor {
    dir_handle: ReadDir,
    pub logs_processed: u128,
}

impl LogsProcessor {
    pub async fn new(logs_dir: &str) -> Result<Self, LogsError> {
        tokio::fs::read_dir(logs_dir)
            .await
            .map_err(|io_error| LogsError::IOError(io_error))
            .map(|dir_handle| LogsProcessor { dir_handle, logs_processed: 0 })
    }

    pub async fn next_log(&mut self) -> Result<Option<BlockLog>, LogsError> {
        let mut next_log: Option<BlockLog> = None;
        while let Some(next_entry) = self.dir_handle
            .next_entry()
            .await
            .map_err(|io_error| LogsError::IOError(io_error))?
        {
            let file_type = next_entry.file_type()
                .await
                .map_err(|io_error| LogsError::IOError(io_error))?;
            
            if !file_type.is_file() { continue; }

            if let Some(file_name) = next_entry
                .file_name()
                .to_str()
                .map(|str| str.to_string()) 
            {
                let fragments: Vec<&str> = file_name
                    .split('.').collect();

                if fragments.len() != 2 { continue; }

                if let (Some(name), Some(extension)) = 
                    ( fragments.get(0).map(|str| str.to_owned())
                    , fragments.get(1).map(|str| str.to_owned())
                    ) 
                {
                    if extension != "json" { continue; }

                    let log_path = next_entry.path();
                    let json = read_block_log(&log_path).await?;
                    self.logs_processed += 1;
                    next_log = Some(BlockLog { log_name: name.to_string(), json });
                    break;
                }
            }
        }

        Ok(next_log)
    }
}

pub async fn read_block_log(log_path: &Path) -> Result<serde_json::Value, LogsError> {
    let mut log_file = File::open(log_path)
        .await
        .map_err(|err| LogsError::IOError(err))?;
    let mut contents = Vec::new();

    log_file.read_to_end(&mut contents)
        .await
        .map_err(|err| LogsError::IOError(err))?;

    let str = unsafe { std::str::from_utf8_unchecked(&contents) };

    let block_log = serde_json::from_str(str)
        .map_err(|err| LogsError::JSONError(err))?;

    Ok(block_log)
}
