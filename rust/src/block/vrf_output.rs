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
