//! Off-chain usernames

use super::Username;
use crate::base::public_key::PublicKey;
use csv::Reader;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct Record {
    public_key: String,
    username: String,
}

#[derive(Default)]
pub struct OffChainUsernames {
    pub usernames: HashMap<PublicKey, Username>,
}

pub const OFF_CHAIN_USERNAME_CONTENTS: &str = include_str!("../../../data/off-chain-usernames.csv");

//////////
// impl //
//////////

impl OffChainUsernames {
    pub fn new() -> anyhow::Result<Self> {
        let mut usernames = HashMap::new();

        let mut rdr = Reader::from_reader(OFF_CHAIN_USERNAME_CONTENTS.as_bytes());
        for result in rdr.deserialize() {
            let record: Record = result?;

            usernames.insert(
                PublicKey::new(record.public_key)?,
                Username::new(record.username)?,
            );
        }

        Ok(OffChainUsernames { usernames })
    }

    pub fn get_off_chain_username(&self, pk: &PublicKey) -> Option<&Username> {
        self.usernames.get(pk)
    }
}

#[cfg(test)]
mod tests {
    use super::OffChainUsernames;
    use crate::base::{public_key::PublicKey, username::Username};

    #[test]
    fn off_chain_username_csv() -> anyhow::Result<()> {
        let off_chain_usernames = OffChainUsernames::new()?;
        let pk = PublicKey::new("B62qpge4uMq4Vv5Rvc8Gw9qSquUYd6xoW1pz7HQkMSHm6h1o7pvLPAN")?;

        assert_eq!(
            *off_chain_usernames.get_off_chain_username(&pk).unwrap(),
            Username::new("MinaExplorer")?
        );
        Ok(())
    }
}
