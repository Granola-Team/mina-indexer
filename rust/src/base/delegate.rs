//! Delegate

use super::public_key::PublicKey;

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Delegate(pub PublicKey);

///////////
// check //
///////////

impl super::check::Check for Delegate {
    fn check(&self, other: &Self) -> bool {
        let check = self != other;
        if check {
            log::error!("Mismatching delegates {} {}", self.0, other.0)
        }

        check
    }
}

/////////////////
// conversions //
/////////////////

impl From<PublicKey> for Delegate {
    fn from(value: PublicKey) -> Self {
        Self(value)
    }
}
