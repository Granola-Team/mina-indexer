use crate::protocol::serialization_types::{
    common::{Base58EncodableVersionedType, HashV1},
    version_bytes,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct EpochSeed(pub String);

impl EpochSeed {
    pub fn from_hashv1(hashv1: HashV1) -> String {
        let seed_bs58: Base58EncodableVersionedType<{ version_bytes::EPOCH_SEED }, _> =
            hashv1.into();
        seed_bs58.to_base58_string().expect("base58 encoded seed")
    }
}
