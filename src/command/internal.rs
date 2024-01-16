use crate::block::precomputed::PrecomputedBlock;
use mina_serialization_types::staged_ledger_diff as mina_rs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum InternalCommand {
    Coinbase {
        coinbase_receiver_balance: u64,
        fee_transfer_receiver_balance: Option<u64>,
    },
    FeeTransfer {
        receiver1_balance: u64,
        receiver2_balance: Option<u64>,
    },
}

impl From<&mina_rs::InternalCommandBalanceData> for InternalCommand {
    fn from(value: &mina_rs::InternalCommandBalanceData) -> Self {
        match value {
            mina_rs::InternalCommandBalanceData::CoinBase(coinbase) => {
                let mina_rs::CoinBaseBalanceData {
                    coinbase_receiver_balance,
                    fee_transfer_receiver_balance,
                } = coinbase.clone().inner();
                Self::Coinbase {
                    coinbase_receiver_balance: coinbase_receiver_balance.inner().inner().inner(),
                    fee_transfer_receiver_balance: fee_transfer_receiver_balance
                        .map(|f| f.inner().inner().inner()),
                }
            }
            mina_rs::InternalCommandBalanceData::FeeTransfer(fee_transfer) => {
                let mina_rs::FeeTransferBalanceData {
                    receiver1_balance,
                    receiver2_balance,
                } = fee_transfer.clone().inner();
                Self::FeeTransfer {
                    receiver1_balance: receiver1_balance.inner().inner().inner(),
                    receiver2_balance: receiver2_balance.map(|b| b.inner().inner().inner()),
                }
            }
        }
    }
}

impl InternalCommand {
    pub fn from_precomputed(block: &PrecomputedBlock) -> Vec<Self> {
        block
            .internal_command_balances()
            .iter()
            .map(Self::from)
            .collect()
    }
}
