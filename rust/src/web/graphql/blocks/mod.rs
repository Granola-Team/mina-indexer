use super::{db, get_block_canonicity};
use crate::{
    block::{self, precomputed::PrecomputedBlock, store::BlockStore},
    command::internal::{InternalCommand, InternalCommandWithData},
    ledger::LedgerHash,
    proof_systems::signer::pubkey::CompressedPubKey,
    protocol::serialization_types::{common::Base58EncodableVersionedType, version_bytes},
    store::{blocks_global_slot_idx_iterator, blocks_global_slot_idx_state_hash_from_entry},
    web::graphql::gen::BlockQueryInput,
};
use async_graphql::{Context, Enum, Object, Result, SimpleObject};
use chrono::{DateTime, SecondsFormat};

#[derive(Default)]
pub struct BlocksQueryRoot;

#[Object]
impl BlocksQueryRoot {
    async fn block<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<BlockQueryInput>,
    ) -> Result<Option<BlockWithCanonicity>> {
        let db = db(ctx);

        // no query filters => get the best block
        if query.is_none() {
            return Ok(db.get_best_block().map(|b| {
                b.map(|pcb| BlockWithCanonicity {
                    canonical: get_block_canonicity(db, &pcb.state_hash),
                    block: pcb.into(),
                })
            })?);
        }

        // Use constant time access if we have state hash
        if let Some(state_hash) = query.clone().and_then(|input| input.state_hash) {
            if !block::is_valid_state_hash(&state_hash) {
                return Ok(None);
            }

            let pcb = match db.get_block(&state_hash.clone().into())? {
                Some(pcb) => pcb,
                None => return Ok(None),
            };
            let block = BlockWithCanonicity {
                canonical: get_block_canonicity(db, &state_hash),
                block: pcb.into(),
            };

            if query.unwrap().matches(&block) {
                return Ok(Some(block));
            }
            return Ok(None);
        }

        // TODO bound query search space if given any inputs

        // else iterate from the end
        for entry in blocks_global_slot_idx_iterator(db, speedb::IteratorMode::End) {
            let state_hash = blocks_global_slot_idx_state_hash_from_entry(&entry)?;
            let pcb = db
                .get_block(&state_hash.clone().into())?
                .expect("block to be returned");
            let block = BlockWithCanonicity {
                canonical: get_block_canonicity(db, &state_hash),
                block: pcb.into(),
            };

            if query.as_ref().map_or(true, |q| q.matches(&block)) {
                return Ok(Some(block));
            }
        }

        Ok(None)
    }

    async fn blocks<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<BlockQueryInput>,
        #[graphql(default = 100)] limit: usize,
        sort_by: Option<BlockSortByInput>,
    ) -> Result<Vec<BlockWithCanonicity>> {
        let db = db(ctx);
        let mut blocks: Vec<BlockWithCanonicity> = Vec::with_capacity(limit);
        let mode = if let Some(BlockSortByInput::BlockHeightAsc) = sort_by {
            speedb::IteratorMode::Start
        } else {
            speedb::IteratorMode::End
        };

        // TODO bound query search space if given any inputs

        for entry in blocks_global_slot_idx_iterator(db, mode) {
            let state_hash = blocks_global_slot_idx_state_hash_from_entry(&entry)?;
            let pcb = db
                .get_block(&state_hash.clone().into())?
                .expect("block to be returned");
            let block = BlockWithCanonicity {
                canonical: get_block_canonicity(db, &state_hash),
                block: pcb.into(),
            };

            if query.as_ref().map_or(true, |q| q.matches(&block)) {
                blocks.push(block);
            }

            if blocks.len() == limit {
                break;
            }
        }

        Ok(blocks)
    }
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum BlockSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,
    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,
}

#[derive(SimpleObject)]
pub struct BlockWithCanonicity {
    /// Value canonical
    pub canonical: bool,
    /// Value block
    #[graphql(flatten)]
    pub block: Block,
}

#[derive(SimpleObject)]
pub struct Block {
    /// Value state_hash
    state_hash: String,
    /// Value block_height
    block_height: u32,
    /// Value winning_account
    winner_account: WinnerAccount,
    /// Value date_time as ISO 8601 string
    date_time: String,
    /// Value received_time as ISO 8601 string
    received_time: String,
    /// Value creator account
    creator_account: CreatorAccount,
    /// Value creator public key
    creator: String,
    /// Value protocol state
    protocol_state: ProtocolState,
    /// Value transaction fees
    tx_fees: String,
    /// Value SNARK fees
    snark_fees: String,
    /// Value transactions
    transactions: Transactions,
}

#[derive(SimpleObject)]
struct Transactions {
    /// Value coinbase
    coinbase: u64,
    /// Value coinbase receiver account
    coinbase_receiver_account: CoinbaseReceiverAccount,
    /// Value fee transfer
    fee_transfer: Vec<BlockFeetransfer>,
}

#[derive(SimpleObject)]
struct BlockFeetransfer {
    pub fee: u64,
    pub recipient: String,
    #[graphql(name = "type")]
    pub feetransfer_kind: String,
}

#[derive(SimpleObject)]
struct CoinbaseReceiverAccount {
    /// Value public key
    public_key: String,
}

#[derive(SimpleObject)]
struct ConsensusState {
    /// Value total currency
    total_currency: u64,
    /// Value block height
    blockchain_length: u32,
    /// Value block height
    block_height: u32,
    /// Value epoch count
    epoch_count: u32,
    /// Value epoch count
    epoch: u32,
    /// Value has ancestors the same checkpoint window
    has_ancestor_in_same_checkpoint_window: bool,
    /// Value last VRF output
    last_vrf_output: String,
    /// Value minimum window density
    min_window_density: u32,
    /// Value current slot
    slot: u32,
    /// Value global slot
    slot_since_genesis: u32,
    /// Value next epoch data
    next_epoch_data: NextEpochData,
    /// Value next epoch data
    staking_epoch_data: StakingEpochData,
}

#[derive(SimpleObject)]
struct StakingEpochData {
    /// Value seed
    seed: String,
    /// Value epoch length
    epoch_length: u32,
    /// Value start checkpoint
    start_checkpoint: String,
    /// Value lock checkpoint
    lock_checkpoint: String,
    /// Value staking ledger
    ledger: StakingEpochDataLedger,
}

#[derive(SimpleObject)]
struct NextEpochData {
    /// Value seed
    seed: String,
    /// Value epoch length
    epoch_length: u32,
    /// Value start checkpoint
    start_checkpoint: String,
    /// Value lock checkpoint
    lock_checkpoint: String,
    /// Value next ledger
    ledger: NextEpochDataLedger,
}

#[derive(SimpleObject)]
struct NextEpochDataLedger {
    /// Value hash
    hash: String,
    /// Value total currency
    total_currency: u64,
}

#[derive(SimpleObject)]
struct StakingEpochDataLedger {
    /// Value hash
    hash: String,
    /// Value total currency
    total_currency: u64,
}

#[derive(SimpleObject)]
struct BlockchainState {
    /// Value utc_date as numeric string
    utc_date: String,
    /// Value date as numeric string
    date: String,
    /// Value snarked ledger hash
    snarked_ledger_hash: String,
    /// Value staged ledger hash
    staged_ledger_hash: String,
}

#[derive(SimpleObject)]
struct ProtocolState {
    /// Value parent state hash
    previous_state_hash: String,
    /// Value blockchain state
    blockchain_state: BlockchainState,
    /// Value consensus state
    consensus_state: ConsensusState,
}

#[derive(SimpleObject)]
struct WinnerAccount {
    /// The public_key for the WinnerAccount
    public_key: String,
}

#[derive(SimpleObject)]
struct CreatorAccount {
    /// The public_key for the CreatorAccount
    public_key: String,
}

/// convert epoch millis to an ISO 8601 formatted date
fn millis_to_date_string(millis: i64) -> String {
    let date_time = DateTime::from_timestamp_millis(millis).unwrap();
    date_time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

impl From<PrecomputedBlock> for Block {
    fn from(block: PrecomputedBlock) -> Self {
        let winner_account = block.block_creator().0;
        let date_time = millis_to_date_string(block.timestamp().try_into().unwrap());
        let pk_creator = block.consensus_state().block_creator;
        let creator = CompressedPubKey::from(&pk_creator).into_address();
        let scheduled_time = block.scheduled_time.clone();
        let received_time = millis_to_date_string(scheduled_time.parse::<i64>().unwrap());
        let previous_state_hash = block.previous_state_hash().0;
        let tx_fees = block.tx_fees();
        let snark_fees = block.snark_fees();
        let utc_date = block
            .protocol_state
            .body
            .t
            .t
            .blockchain_state
            .t
            .t
            .timestamp
            .t
            .t
            .to_string();

        let blockchain_state = block.protocol_state.body.t.t.blockchain_state.clone().t.t;
        let snarked_ledger_hash =
            LedgerHash::from_hashv1(blockchain_state.clone().snarked_ledger_hash).0;
        let staged_ledger_hashv1 = blockchain_state
            .staged_ledger_hash
            .t
            .t
            .non_snark
            .t
            .ledger_hash;
        let staged_ledger_hash = LedgerHash::from_hashv1(staged_ledger_hashv1).0;

        // consensus state
        let consensus_state = block.protocol_state.body.t.t.consensus_state.clone().t.t;

        let total_currency = consensus_state.total_currency.t.t;
        let blockchain_length = block.blockchain_length;
        let block_height = blockchain_length;
        let epoch_count = consensus_state.epoch_count.t.t;
        let epoch = epoch_count;
        let has_ancestor_in_same_checkpoint_window =
            consensus_state.has_ancestor_in_same_checkpoint_window;
        let last_vrf_output = block.last_vrf_output();
        let min_window_density = consensus_state.min_window_density.t.t;
        let slot_since_genesis = consensus_state.global_slot_since_genesis.t.t;
        let slot = consensus_state.curr_global_slot.t.t.slot_number.t.t;

        // NextEpochData
        let seed_hashv1 = consensus_state.next_epoch_data.t.t.seed;
        let seed_bs58: Base58EncodableVersionedType<{ version_bytes::EPOCH_SEED }, _> =
            seed_hashv1.into();
        let seed = seed_bs58.to_base58_string().expect("bs58 encoded seed");
        let epoch_length = consensus_state.next_epoch_data.t.t.epoch_length.t.t;

        let start_checkpoint_hashv1 = consensus_state.next_epoch_data.t.t.start_checkpoint;
        let start_checkpoint_bs58: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            start_checkpoint_hashv1.into();
        let start_checkpoint = start_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded start checkpoint");

        let lock_checkpoint_hashv1 = consensus_state.next_epoch_data.t.t.lock_checkpoint;
        let lock_checkpoint_bs58: Base58EncodableVersionedType<{ version_bytes::STATE_HASH }, _> =
            lock_checkpoint_hashv1.into();
        let lock_checkpoint = lock_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded lock checkpoint");

        let ledger_hashv1 = consensus_state.next_epoch_data.t.t.ledger.t.t.hash;
        let ledger_hash_bs58: Base58EncodableVersionedType<{ version_bytes::LEDGER_HASH }, _> =
            ledger_hashv1.into();
        let ledger_hash = ledger_hash_bs58
            .to_base58_string()
            .expect("bs58 encoded ledger hash");
        let ledger_total_currency = consensus_state
            .next_epoch_data
            .t
            .t
            .ledger
            .t
            .t
            .total_currency
            .t
            .t;

        // StakingEpochData
        let staking_seed_hashv1 = consensus_state.staking_epoch_data.t.t.seed;
        let staking_seed_bs58: Base58EncodableVersionedType<{ version_bytes::EPOCH_SEED }, _> =
            staking_seed_hashv1.into();
        let staking_seed = staking_seed_bs58
            .to_base58_string()
            .expect("bs58 encoded seed");

        let staking_epoch_length = consensus_state.staking_epoch_data.t.t.epoch_length.t.t;

        let staking_start_checkpoint_hashv1 =
            consensus_state.staking_epoch_data.t.t.start_checkpoint;
        let staking_start_checkpoint_bs58: Base58EncodableVersionedType<
            { version_bytes::STATE_HASH },
            _,
        > = staking_start_checkpoint_hashv1.into();
        let staking_start_checkpoint = staking_start_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded start checkpoint");

        let staking_lock_checkpoint_hashv1 = consensus_state.staking_epoch_data.t.t.lock_checkpoint;
        let staking_lock_checkpoint_bs58: Base58EncodableVersionedType<
            { version_bytes::STATE_HASH },
            _,
        > = staking_lock_checkpoint_hashv1.into();
        let staking_lock_checkpoint = staking_lock_checkpoint_bs58
            .to_base58_string()
            .expect("bs58 encoded lock checkpoint");

        let staking_ledger_hashv1 = consensus_state.staking_epoch_data.t.t.ledger.t.t.hash;
        let staking_ledger_hash_bs58: Base58EncodableVersionedType<
            { version_bytes::LEDGER_HASH },
            _,
        > = staking_ledger_hashv1.into();
        let staking_ledger_hash = staking_ledger_hash_bs58
            .to_base58_string()
            .expect("bs58 encoded ledger hash");
        let staking_ledger_total_currency = consensus_state
            .staking_epoch_data
            .t
            .t
            .ledger
            .t
            .t
            .total_currency
            .t
            .t;
        let coinbase_receiver_account = block.coinbase_receiver().0;
        let supercharged = consensus_state.supercharge_coinbase;

        let coinbase: u64 = if supercharged {
            1440000000000
        } else {
            720000000000
        };

        let fee_transfers: Vec<BlockFeetransfer> = InternalCommand::from_precomputed(&block)
            .into_iter()
            .map(|cmd| InternalCommandWithData::from_internal_cmd(cmd, &block))
            .map(|ft| ft.into())
            .collect();
        Block {
            state_hash: block.state_hash,
            block_height: block.blockchain_length,
            date_time,
            winner_account: WinnerAccount {
                public_key: winner_account,
            },
            creator_account: CreatorAccount {
                public_key: creator.clone(),
            },
            creator,
            received_time,
            protocol_state: ProtocolState {
                previous_state_hash,
                blockchain_state: BlockchainState {
                    date: utc_date.clone(),
                    utc_date,
                    snarked_ledger_hash,
                    staged_ledger_hash,
                },
                consensus_state: ConsensusState {
                    total_currency,
                    blockchain_length,
                    block_height,
                    epoch,
                    epoch_count,
                    has_ancestor_in_same_checkpoint_window,
                    last_vrf_output,
                    min_window_density,
                    slot,
                    slot_since_genesis,
                    next_epoch_data: NextEpochData {
                        seed,
                        epoch_length,
                        start_checkpoint,
                        lock_checkpoint,
                        ledger: NextEpochDataLedger {
                            hash: ledger_hash,
                            total_currency: ledger_total_currency,
                        },
                    },
                    staking_epoch_data: StakingEpochData {
                        seed: staking_seed,
                        epoch_length: staking_epoch_length,
                        start_checkpoint: staking_start_checkpoint,
                        lock_checkpoint: staking_lock_checkpoint,
                        ledger: StakingEpochDataLedger {
                            hash: staking_ledger_hash,
                            total_currency: staking_ledger_total_currency,
                        },
                    },
                },
            },
            tx_fees: tx_fees.to_string(),
            snark_fees: snark_fees.to_string(),
            transactions: Transactions {
                coinbase,
                coinbase_receiver_account: CoinbaseReceiverAccount {
                    public_key: coinbase_receiver_account,
                },
                fee_transfer: fee_transfers,
            },
        }
    }
}

impl BlockQueryInput {
    pub fn matches(&self, block: &BlockWithCanonicity) -> bool {
        let mut matches = true;
        if let Some(state_hash) = &self.state_hash {
            matches = matches && &block.block.state_hash == state_hash;
        }
        if let Some(canonical) = &self.canonical {
            matches = matches && &block.canonical == canonical;
        }
        if let Some(creator_account) = &self.creator_account {
            if let Some(public_key) = creator_account.public_key.as_ref() {
                matches = matches && block.block.creator_account.public_key == *public_key;
            }
        }
        if let Some(query) = &self.and {
            matches = matches && query.iter().all(|and| and.matches(block));
        }
        if let Some(query) = &self.or {
            if !query.is_empty() {
                matches = matches && query.iter().any(|or| or.matches(block));
            }
        }
        matches
    }
}

impl From<InternalCommandWithData> for BlockFeetransfer {
    fn from(int_cmd: InternalCommandWithData) -> Self {
        match int_cmd {
            InternalCommandWithData::FeeTransfer {
                receiver,
                amount,
                kind,
                ..
            } => Self {
                fee: amount,
                recipient: receiver.0,
                feetransfer_kind: kind.to_string(),
            },
            InternalCommandWithData::Coinbase {
                receiver,
                amount,
                kind,
                ..
            } => Self {
                fee: amount,
                recipient: receiver.0,
                feetransfer_kind: kind.to_string(),
            },
        }
    }
}
