//! Username store helpers

use super::common::{pk_index_key, U32_LEN};
use crate::base::public_key::PublicKey;

/// Use with [username_cf]
pub fn username_key(pk: &PublicKey, index: u32) -> [u8; PublicKey::LEN + U32_LEN] {
    pk_index_key(pk, index)
}
