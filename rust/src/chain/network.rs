//! Network

use bincode::{Decode, Encode};
use clap::builder::OsStr;
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, PartialEq, Eq, Encode, Decode, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[default]
    Mainnet,
    Devnet,
    Testworld,
    Berkeley,
    Custom(String),
}

impl Network {
    const MAINNET: &'static str = "mainnet";
    const DEVNET: &'static str = "devnet";
    const TESTWORLD: &'static str = "testworld";
    const BERKELEY: &'static str = "berkeley";
}

/////////////////
// conversions //
/////////////////

impl From<Vec<u8>> for Network {
    fn from(value: Vec<u8>) -> Self {
        String::from_utf8(value).unwrap().as_str().into()
    }
}

impl From<&str> for Network {
    fn from(value: &str) -> Self {
        match value {
            Self::MAINNET => Self::Mainnet,
            Self::DEVNET => Self::Devnet,
            Self::TESTWORLD => Self::Testworld,
            Self::BERKELEY => Self::Berkeley,
            network => Self::Custom(network.to_string()),
        }
    }
}

impl From<Network> for OsStr {
    fn from(value: Network) -> Self {
        value.to_string().into()
    }
}

///////////////////
// debug/display //
///////////////////

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Network::Mainnet => Network::MAINNET,
                Network::Devnet => Network::DEVNET,
                Network::Testworld => Network::TESTWORLD,
                Network::Berkeley => Network::BERKELEY,
                Network::Custom(network) => network,
            }
        )
    }
}

impl std::fmt::Debug for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
