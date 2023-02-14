use super::PublicKey;

pub struct Transaction {
    pub source: PublicKey,
    pub receiver: PublicKey,
    pub amount: u64,
}
