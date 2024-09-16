#[derive(Clone)]
pub struct TxnHash(pub String);

impl TxnHash {
    pub const LEN: usize = 54;
}
