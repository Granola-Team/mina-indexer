use serde_json::{Map, Value};

use crate::state::ledger::PublicKey;

pub mod reader;

pub struct BlockLog {
    pub state_hash: String,
    pub json: Value,
}

fn get_consensus_state(block_log: &BlockLog) -> Option<Map<String, Value>> {
    block_log
        .json
        .as_object()?
        .get("protocol_state")?
        .as_object()?
        .get("body")?
        .as_object()?
        .get("consensus_state")?
        .as_object()
        .cloned()
}

fn get_block_creator(consensus_state: &Map<String, Value>) -> Option<PublicKey> {
    Some(consensus_state.get("block_creator")?.as_str()?.to_string())
}

fn get_block_stake_winner(consensus_state: &Map<String, Value>) -> Option<PublicKey> {
    Some(
        consensus_state
            .get("block_stake_winner")?
            .as_str()?
            .to_string(),
    )
}

fn get_coinbase_receiver(consensus_state: &Map<String, Value>) -> Option<PublicKey> {
    Some(
        consensus_state
            .get("coinbase_receiver")?
            .as_str()?
            .to_string(),
    )
}

pub fn get_block_commands(block_log: &BlockLog) -> Option<&Vec<Value>> {
    block_log
        .json
        .as_object()?
        .get("staged_ledger_diff")?
        .as_object()?
        .get("diff")?
        .as_array()?
        .get(0)?
        .as_object()?
        .get("commands")?
        .as_array()
}

pub fn public_keys_seen(block_log: &BlockLog) -> Vec<PublicKey> {
    let mut public_keys = Vec::new();
    if let Some(consensus_state) = get_consensus_state(block_log) {
        if let Some(block_stake_winner) = get_block_stake_winner(&consensus_state) {
            public_keys.push(block_stake_winner);
        }
        if let Some(block_creator) = get_block_creator(&consensus_state) {
            public_keys.push(block_creator);
        }
        if let Some(coinbase_receiver) = get_coinbase_receiver(&consensus_state) {
            public_keys.push(coinbase_receiver);
        }
    }

    if let Some(commands) = get_block_commands(block_log) {
        commands
            .iter()
            .filter_map(|command| {
                let payload_body = command
                    .as_object()?
                    .get("data")?
                    .as_array()?
                    .get(1)?
                    .as_object()?
                    .get("payload")?
                    .as_object()?
                    .get("body")?
                    .as_array()?
                    .get(1)?
                    .as_object()?;

                let source_pk = payload_body.get("source_pk")?.as_str()?.to_string();
                let receiver_pk = payload_body.get("receiver_pk")?.as_str()?.to_string();

                Some(vec![source_pk, receiver_pk])
            })
            .flatten()
            .for_each(|public_key| {
                public_keys.push(public_key);
            });
    }

    public_keys
}
