use base64::{
    alphabet,
    engine::{self, Engine},
};
use blake2::{
    digest::{Update, VariableOutput},
    Blake2bVar,
};

#[derive(
    Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
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

    pub fn base64_decode(input: &str) -> anyhow::Result<VrfOutput> {
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

#[cfg(test)]
mod test {
    use super::*;

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
