#[derive(Clone)]
pub struct TxnHash(pub String);

impl TxnHash {
    pub const LEN: usize = 53;
    pub const PREFIX: &'static str = "Ckp";
}
