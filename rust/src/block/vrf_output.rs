use base64::{
    alphabet,
    engine::{self, Engine},
};
use blake2::{
    digest::{Update, VariableOutput},
    Blake2bVar,
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct VrfOutput(Vec<u8>);

impl VrfOutput {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn base64_encode(&self) -> String {
        let b64 =
            engine::GeneralPurpose::new(&alphabet::URL_SAFE, engine::GeneralPurposeConfig::new());
        b64.encode(self.0.as_slice())
    }

    pub fn base64_decode(input: &str) -> anyhow::Result<Self> {
        let b64 =
            engine::GeneralPurpose::new(&alphabet::URL_SAFE, engine::GeneralPurposeConfig::new());
        Ok(Self(b64.decode(input)?))
    }

    pub fn hex_digest(&self) -> Vec<u8> {
        let mut hasher = Blake2bVar::new(32).unwrap();
        hasher.update(self.0.as_slice());
        hasher.finalize_boxed().to_vec()
    }
}

///////////
// serde //
///////////

impl Serialize for VrfOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::utility::serde::to_str(self, serializer)
    }
}

impl<'de> Deserialize<'de> for VrfOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        crate::utility::serde::from_str(deserializer)
    }
}

/////////////////
// conversions //
/////////////////

impl std::str::FromStr for VrfOutput {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::base64_decode(s)
    }
}

/////////////
// default //
/////////////

impl Default for VrfOutput {
    fn default() -> Self {
        Self([0; 32].to_vec())
    }
}

/////////////
// display //
/////////////

impl Display for VrfOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", Self::base64_encode(self))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn roundtrip() -> anyhow::Result<()> {
        let vrf = VrfOutput::default();
        let vrf_str = vrf.to_string();

        // serialize
        let ser = serde_json::to_vec(&vrf)?;

        // deserialize
        let res: VrfOutput = serde_json::from_slice(&ser)?;

        // roundtrip
        assert_eq!(vrf, res);

        // same serialization as string
        assert_eq!(ser, serde_json::to_vec(&vrf_str)?);

        Ok(())
    }

    #[test]
    fn last_pre_hardfork_vrf_output() -> anyhow::Result<()> {
        let encoded = "EiRsdutXWv2DsYeVnFfkddR7C9U1mPFAjzgqA8kNLPdzUDMr3Lesb";
        let decoded = bs58::decode(encoded).into_vec()?;
        let b64 =
            engine::GeneralPurpose::new(&alphabet::URL_SAFE, engine::GeneralPurposeConfig::new());

        assert_eq!(
            b64.encode(&decoded),
            "FQEggPlrr0gYowIPqLsTL_2D9h5ARrW6BFYXxxy2g8mTAgBW-lBi"
        );
        Ok(())
    }

    #[test]
    fn hardfork_genesis_vrf_output() -> anyhow::Result<()> {
        let encoded = "48G7Db7Fbo1DdChse1jcRWowVdM7RvBNXKHKP1UfPhsNBfAQbF8E";
        let decoded = bs58::decode(encoded).into_vec()?;
        let b64 =
            engine::GeneralPurpose::new(&alphabet::URL_SAFE, engine::GeneralPurposeConfig::new());

        assert_eq!(
            b64.encode(&decoded),
            "FSBXKqZKgSiy1T6SsjbrT0i84oDkBpUVsLH1zRviuIj0DjuGEXs="
        );
        Ok(())
    }
}
