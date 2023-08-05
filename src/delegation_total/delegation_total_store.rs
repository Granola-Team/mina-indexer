use super::DelegationTotal;

pub trait DelegationTotalStore {
    fn get_delegation_total(
        &self,
        epoch: u32,
        public_key: &str,
    ) -> anyhow::Result<Option<DelegationTotal>>;
    fn add_delegation_total(
        &self,
        epoch: u32,
        public_key: &str,
        delegation_total: DelegationTotal,
    ) -> anyhow::Result<()>;
}
