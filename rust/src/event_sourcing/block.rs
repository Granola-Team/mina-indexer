use super::models::{CommandSummary, CompletedWorksNanomina, FeeTransfer, FeeTransferViaCoinbase};
use sonic_rs::{JsonValueTrait, Value};
use std::collections::HashMap;

pub trait BlockTrait {
    fn get_snark_work(&self) -> Vec<CompletedWorksNanomina>;
    fn get_user_commands(&self) -> Vec<CommandSummary>;
    fn get_coinbase_receiver(&self) -> String;
    fn get_coinbases(&self) -> Vec<Vec<Value>>;

    fn get_snark_work_count(&self) -> usize {
        self.get_snark_work().len()
    }

    fn get_user_commands_count(&self) -> usize {
        self.get_user_commands().len()
    }

    fn get_excess_block_fees(&self) -> u64 {
        let total_snark_fees = self.get_snark_work().iter().map(|ft| ft.fee_nanomina).sum::<u64>();

        let mut total_fees_paid_into_block_pool = self.get_user_commands().iter().map(|uc| uc.fee_nanomina).sum::<u64>();
        for ftvc in self.get_fee_transfers_via_coinbase().unwrap_or_default().iter() {
            total_fees_paid_into_block_pool += (ftvc.fee * 1_000_000_000f64) as u64;
        }
        total_fees_paid_into_block_pool.saturating_sub(total_snark_fees)
    }

    fn get_fee_transfers(&self) -> Vec<FeeTransfer> {
        let excess_block_fees = self.get_excess_block_fees();
        let mut fee_transfers: HashMap<String, u64> = HashMap::new();
        if excess_block_fees > 0 {
            fee_transfers.insert(self.get_coinbase_receiver(), excess_block_fees);
        }
        for completed_work in self.get_snark_work() {
            *fee_transfers.entry(completed_work.prover).or_insert(0_u64) += completed_work.fee_nanomina;
        }

        // If the fee for a completed work is higher than the available fees, the remainder
        // is allotted out of the coinbase via a fee transfer via coinbase
        for ftvc in self.get_fee_transfers_via_coinbase().unwrap_or_default().iter() {
            let ftvc_fee_nanomina = (ftvc.fee * 1_000_000_000f64) as u64;

            if let Some(current_fee) = fee_transfers.get_mut(&ftvc.receiver) {
                if *current_fee > ftvc_fee_nanomina {
                    *current_fee -= ftvc_fee_nanomina;
                } else {
                    fee_transfers.remove(&ftvc.receiver);
                }
            }
        }

        fee_transfers.retain(|_, v| *v > 0u64);
        fee_transfers
            .into_iter()
            .map(|(prover, fee_nanomina)| FeeTransfer {
                recipient: prover,
                fee_nanomina,
            })
            .collect()
    }

    fn get_internal_command_count(&self) -> usize {
        // Get fee transfers and remove those also in fee transfers via coinbase
        let fee_transfers = self.get_fee_transfers();
        let fee_transfers_via_coinbase = self.get_fee_transfers_via_coinbase().unwrap_or_default();

        fee_transfers.len() + fee_transfers_via_coinbase.len() + 1 // +1 for coinbase
    }

    fn get_total_command_count(&self) -> usize {
        self.get_internal_command_count() + self.get_user_commands_count()
    }

    fn get_fee_transfers_via_coinbase(&self) -> Option<Vec<FeeTransferViaCoinbase>> {
        let fee_transfers = self
            .get_coinbases()
            .iter()
            .filter_map(|coinbase| {
                if coinbase.first().map_or(false, |v| v == "One" || v == "Two") {
                    let v2 = coinbase.last().unwrap();
                    if !v2.is_object() || v2.is_null() {
                        return None;
                    }

                    // Try to extract "receiver_pk" and "fee"
                    let receiver = v2.get("receiver_pk")?.as_str()?.to_string();
                    let fee = v2.get("fee")?.as_str()?.parse::<f64>().ok()?;

                    Some(FeeTransferViaCoinbase { receiver, fee })
                } else {
                    None
                }
            })
            .collect::<Vec<FeeTransferViaCoinbase>>();

        if fee_transfers.is_empty() {
            None
        } else {
            Some(fee_transfers)
        }
    }
}
