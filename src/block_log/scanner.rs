use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use glob::{glob, Paths};

pub enum ScannerRecursion {
    Flat,
    Recursive,
    //Depth(usize),
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct BlockLogEntry {
    pub state_hash: String,
    pub log_path: PathBuf,
}

pub struct LogScanner {
    pub base_dir: PathBuf,
    pub recursion: ScannerRecursion,
    paths: Paths,
}

impl LogScanner {
    pub fn new(base_dir: &Path) -> Self {
        Self::new_internal(base_dir, ScannerRecursion::Flat)
    }

    pub fn new_recursive(base_dir: &Path) -> Self {
        Self::new_internal(base_dir, ScannerRecursion::Recursive)
    }

    // pub fn with_depth(base_dir: &Path, max_depth: usize) -> Self {
    //     Self::new_internal(base_dir, ScannerRecursion::Depth(max_depth))
    // }

    fn new_internal(base_dir: &Path, recursion: ScannerRecursion) -> Self {
        let pattern = match &recursion {
            ScannerRecursion::Flat => format!("{}/*.json", base_dir.display()),
            ScannerRecursion::Recursive => format!("{}/**/*.json", base_dir.display()),
            // ScannerRecursion::Depth(max_depth) => todo!(),
        };
        let base_dir = base_dir.to_owned();
        let paths = glob(&pattern).expect("Failed to read glob pattern");
        Self {
            base_dir,
            recursion,
            paths,
        }
    }

    pub fn log_files(self) -> impl Iterator<Item = BlockLogEntry> {
        self.paths
            .into_iter()
            .filter_map(|path| path.ok())
            .filter(|path_buf| has_state_hash_and_json_filetype(path_buf))
            .filter_map(log_path_to_log_entry)
    }
}

fn log_path_to_log_entry(log_path: PathBuf) -> Option<BlockLogEntry> {
    let state_hash = get_state_hash(log_path.file_name()?)?;
    Some(BlockLogEntry {
        state_hash,
        log_path,
    })
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

    use super::has_state_hash_and_json_filetype;

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
    fn invalid_filenames_are_false() {
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
    fn valid_filenames_are_true() {
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
