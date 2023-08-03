use super::StakingLedger;

pub trait StakingLedgerStore {
    fn add_epoch(&self, epoch: u32, ledger: &StakingLedger) -> anyhow::Result<()>;
    fn get_epoch(&self, epoch_number: u32) -> anyhow::Result<Option<StakingLedger>>;
}
