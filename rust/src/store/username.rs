use crate::ledger::public_key::PublicKey;

pub trait UsernameStore {
    /// Get the username associated with `pk`
    fn get_username(&self, pk: &PublicKey) -> anyhow::Result<Option<String>>;

    /// Set `pk`'s username to `username`
    fn set_username(&self, pk: &PublicKey, username: String) -> anyhow::Result<()>;
}
