//! Simplified Zkapp representation for internal use in ledger calculations

use crate::protocol::serialization_types::staged_ledger_diff as mina_rs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub struct ZkappCommand {
    // TODO
}

/////////////////
// Conversions //
/////////////////

impl From<mina_rs::ZkappCommand> for ZkappCommand {
    fn from(_value: mina_rs::ZkappCommand) -> Self {
        todo!()
    }
}
