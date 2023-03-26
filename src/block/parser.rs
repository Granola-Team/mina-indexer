use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use glob::{glob, Paths};
use tokio::io::AsyncReadExt;

use super::precomputed::{BlockLogContents, PrecomputedBlock};

pub enum SearchRecursion {
    None,
    Recursive,
}

pub struct BlockParser {
    pub log_path: PathBuf,
    pub recursion: SearchRecursion,
    paths: Paths,
}

impl BlockParser {
    pub fn new(log_path: &Path) -> anyhow::Result<Self> {
        Self::new_internal(log_path, SearchRecursion::None)
    }

    pub fn new_recursive(log_path: &Path) -> anyhow::Result<Self> {
        Self::new_internal(log_path, SearchRecursion::Recursive)
    }

    fn new_internal(log_path: &Path, recursion: SearchRecursion) -> anyhow::Result<Self> {
        if log_path.exists() {
            let pattern = match &recursion {
                SearchRecursion::None => format!("{}/*.json", log_path.display()),
                SearchRecursion::Recursive => format!("{}/**/*.json", log_path.display()),
            };
            let log_path = log_path.to_owned();
            let paths = glob(&pattern).expect("Failed to read glob pattern");
            Ok(Self {
                log_path,
                recursion,
                paths,
            })
        } else {
            Err(anyhow::Error::msg(format!(
                "[BlockParser::new_internal] log path {log_path:?} does not exist!"
            )))
        }
    }

    pub async fn next(&mut self) -> anyhow::Result<Option<PrecomputedBlock>> {
        if let Some(next) = self.paths.next() {
            let next_path = next?;
            if has_state_hash_and_json_filetype(&next_path) {
                let blockchain_length =
                    get_blockchain_length(next_path.file_name().expect("filename already checked"));
                let state_hash =
                    get_state_hash(next_path.file_name().expect("filename already checked"))
                        .expect("state hash already checked");

                let mut log_file = tokio::fs::File::open(&next_path).await?;
                let mut log_file_contents = Vec::new();

                log_file.read_to_end(&mut log_file_contents).await?;

                let precomputed_block = PrecomputedBlock::from_log_contents(BlockLogContents {
                    state_hash,
                    blockchain_length,
                    contents: log_file_contents,
                })?;

                return Ok(Some(precomputed_block));
            }
        }

        Ok(None)
    }

    /// get the precomputed block with supplied hash
    /// it must exist ahead of the current block parser file
    #[async_recursion::async_recursion]
    pub async fn get_precomputed_block(
        &mut self,
        state_hash: &str,
    ) -> anyhow::Result<PrecomputedBlock> {
        match self.next().await.unwrap() {
            None => Err(anyhow::Error::msg(format!(
                "
[BlockPasrser::get_precomputed_block]
    Looking in log path: {:?}
    Did not find state hash: {state_hash}
    It may have been skipped unintentionally!
    BlockParser::next() does not exactly follow filename order!",
                self.log_path
            ))),
            Some(precomputed) => Ok(if precomputed.state_hash == state_hash {
                precomputed
            } else {
                self.get_precomputed_block(state_hash).await.unwrap()
            }),
        }
    }
}

/// extract a state hash from an OS file name
fn get_state_hash(file_name: &OsStr) -> Option<String> {
    let last_part = file_name.to_str()?.split('-').last()?.to_string();
    if last_part.starts_with('.') {
        return None;
    }
    if !last_part.starts_with("3N") {
        return None;
    }
    let state_hash = last_part.split('.').next()?;
    if state_hash.contains('.') {
        return None;
    }
    Some(state_hash.to_string())
}

/// extract a blockchain length from an OS file name
fn get_blockchain_length(file_name: &OsStr) -> Option<u32> {
    file_name
        .to_str()?
        .split('-')
        .fold(None, |acc, x| match x.parse::<u32>() {
            Err(_) => acc,
            Ok(x) => Some(x),
        })
}

fn has_state_hash_and_json_filetype(path: &Path) -> bool {
    let file_name = path.file_name();
    if let Some(file_name) = file_name {
        get_state_hash(file_name).is_some()
            && file_name
                .to_str()
                .map(|file_name| file_name.ends_with(".json"))
                .unwrap_or(false)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use std::{ffi::OsString, path::PathBuf};

    use super::{get_blockchain_length, has_state_hash_and_json_filetype};

    const FILENAMES_VALID: [&'static str; 23] = [
        "mainnet-113512-3NK9bewd5kDxzB5Kvyt8niqyiccbb365B2tLdEC2u9e8tG36ds5u.json",
        "mainnet-113518-3NLQ2Zop9dfDKvffNg9EBzSmBqyjYgCi2E1zAuLGFzUfJk6uq7YK.json",
        "mainnet-175222-3NKn7ZtT6Axw3hK3HpyUGRxmirkuUhtR4cYzWFk75NCgmjCcqPby.json",
        "mainnet-179591-3NLNMihHhdxEj78r88mK9JGTdyYuUWTP2hHD4yzJ4CvypjqYd2hv.json",
        "mainnet-179594-3NLBTeqaKMdY94Nu1QSnYMhq6qBSELH2HNJw4z8dYEXaJwgwnKey.json",
        "mainnet-195769-3NKbdBu8uaP41gnp2W2kSyEBDpYKqaSCxMdspoANXboxALK2g2Px.json",
        "mainnet-195770-3NK7CQdrzY5RBw9ugVjeQ2K6nR6dZSckP3Hrf18bopVg2LY8yrMy.json",
        "mainnet-196577-3NKPcXyRq9Ywe5e519n1DCNCNuY6fdDukuWXwrY4oWkDzdf3WWsF.json",
        "mainnet-206418-3NKS1csVgEyHj4sSeK2mi6aD2oCy5jYVd2ANhNT7ydo7oy1b5mYu.json",
        "mainnet-216651-3NLp9p3X8oF1ydSC1MgXnB99iJoSTTCV4qs4urmTKfiWTd6BbBsL.json",
        "mainnet-220897-3NL4HLb7MQrxmAqVw8D4vEXCj2tdT8zgP9DFWGRoDxP72b4wxyUw.json",
        "mainnet-2-3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH.json",
        "mainnet-3NK2upcz2s6BmmoD6btjtJqSw1wNdyM9H5tXSD9nmN91mQMe4vH8.json",
        "mainnet-3NK2uq5kh6PwbUEwmhwR5RHfJNBgbwvwxxHQnKtQN5aYANudn3Wx.json",
        "mainnet-3NK2veoFnf9dKkqU7DUg4dAgQnapNaQUZZHHANK3kqaimKD1vFuv.json",
        "mainnet-3NK2xHq4mq5mBEG6jNhWTKSycG315pHwnZKdPqGYiyY58N3tn4oJ.json",
        "mainnet-3NK3c24DBH1aA83x3fhQLMC9UwFRUWVtFJG57o94MsDRqyDvR7us.json",
        "mainnet-40702-3NLkEG6S6Ra8Z1i5U5MPSNWV13hzQV8pYx1xBaeLDFN4EJhSuksw.json",
        "mainnet-750-3NLFkhrNBLRxh8cfCAHEFJSe29MEuT3HGNEcheXBKvexfRuEo9eC.json",
        "mainnet-84160-3NKJCCUhCqpueErQWmPMh67gk8uCY8ttFAK6bqG9xyF26rzjZBJ5.json",
        "mainnet-84161-3NK8iBQSkCQtCpnm2qWCvhixuEsiHQq7SL7YY31nyXkiLGEDMyGk.json",
        "mainnet-9638-3NL51H2ZPJUvuSFBaR56cEMqSt1ytiPpoHx7e6aQgEFNsVUPxSAn.json",
        "mainnet-9644-3NK4apiDvnT4ywWEw6KBEk1UzTd1XK7SGXFZDVC9GPCDaT3EXdsv.json",
    ];

    const FILENAMES_INVALID: [&'static str; 6] = [
        "mainnet-113512-3NK9bewd5kDxzB5Kvyt8niqyiccbb365B2tLdEC2u9e8tG36ds5u",
        "mainnet-113518-3NLQ2Zop9dfDKvffNg9EBzSmBqyjYgCi2E1zAuLGFzUfJk6uq7YK.j",
        "mainnet-175222.json",
        "LNMihHhdxEj78r88mK9JGTdyYuUWTP2hHD4yzJ4CvypjqYd2hv.json",
        "mainnet.json",
        "mainnet-195769-.json",
    ];

    #[test]
    fn blockchain_lengths_valid_or_default_none() {
        Vec::from(FILENAMES_VALID)
            .into_iter()
            .map(OsString::from)
            .map(|x| get_blockchain_length(&x))
            .for_each(|x| {
                println!("{:?}", x);
            });
        Vec::from(FILENAMES_INVALID)
            .into_iter()
            .map(OsString::from)
            .map(|x| get_blockchain_length(&x))
            .for_each(|x| {
                println!("{:?}", x);
            });
    }

    #[test]
    fn invalid_filenames_have_invalid_state_hash_or_non_json_extension() {
        Vec::from(FILENAMES_INVALID)
            .into_iter()
            .map(OsString::from)
            .map(|os_string| {
                (
                    os_string.clone(),
                    has_state_hash_and_json_filetype(&PathBuf::from(os_string)),
                )
            })
            .for_each(|(os_string, result)| {
                dbg!(os_string);
                assert!(result == false)
            });
    }

    #[test]
    fn valid_filenames_have_valid_state_hash_and_json_extension() {
        Vec::from(FILENAMES_VALID)
            .into_iter()
            .map(OsString::from)
            .map(|os_string| {
                (
                    os_string.clone(),
                    has_state_hash_and_json_filetype(&PathBuf::from(os_string)),
                )
            })
            .for_each(|(os_string, result)| {
                dbg!(os_string);
                assert!(result == true)
            });
    }
}
