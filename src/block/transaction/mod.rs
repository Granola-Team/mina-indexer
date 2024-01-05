pub struct Transaction {
    pub hash: String,
    pub state_hash: String,
    pub public_keys: Vec<String>,
}

// TODO precomputed block -> Vec<Transaction>
