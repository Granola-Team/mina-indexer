use serde_json::Value;
use std::path::PathBuf;
use tokio::fs::{File, ReadDir};
use tokio::io::AsyncReadExt;

#[derive(Debug)]
pub enum LogError {
    IOError(std::io::Error),
    JSONError(serde_json::Error),
}

pub struct FilesystemJSONReader {
    dir_handle: ReadDir,
    pub logs_processed: u128,
}

impl FilesystemJSONReader {
    pub async fn new(logs_dir: &str) -> Result<Self, LogError> {
        tokio::fs::read_dir(logs_dir)
            .await
            .map_err(|io_error| LogError::IOError(io_error))
            .map(|dir_handle| FilesystemJSONReader {
                dir_handle,
                logs_processed: 0,
            })
    }

    pub async fn next_log_data(&mut self) -> Result<Option<(String, PathBuf)>, LogError> {
        let mut next_log_data: Option<(String, PathBuf)> = None;
        while let Some(next_entry) = self
            .dir_handle
            .next_entry()
            .await
            .map_err(|io_error| LogError::IOError(io_error))?
        {
            let file_type = next_entry
                .file_type()
                .await
                .map_err(|io_error| LogError::IOError(io_error))?;

            if !file_type.is_file() {
                continue;
            }

            if let Some(file_name) = next_entry.file_name().to_str().map(|str| str.to_string()) {
                let fragments: Vec<&str> = file_name.split('.').collect();

                if fragments.len() != 2 {
                    continue;
                }

                if let (Some(log_name), Some(extension)) = (
                    fragments.get(0).map(|str| str.to_owned()),
                    fragments.get(1).map(|str| str.to_owned()),
                ) {
                    if extension != "json" {
                        continue;
                    }

                    let state_hash = log_name
                        .split('-')
                        .last()
                        .expect("log name has a state hash")
                        .to_string();

                    next_log_data = Some((state_hash, next_entry.path()));
                    break;
                }
            }
        }

        Ok(next_log_data)
    }

    pub async fn read_block_log(log_path: PathBuf) -> Result<Value, LogError> {
        let mut log_file = File::open(log_path)
            .await
            .map_err(|err| LogError::IOError(err))?;
        let mut contents = Vec::new();

        log_file
            .read_to_end(&mut contents)
            .await
            .map_err(|err| LogError::IOError(err))?;

        let str = unsafe { std::str::from_utf8_unchecked(&contents) };

        let block_log = serde_json::from_str(str).map_err(|err| LogError::JSONError(err))?;

        Ok(block_log)
    }
}
