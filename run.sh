#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail
if [[ "${TRACE-0}" == "1" ]]; then
    set -o xtrace
fi

cd "$(dirname "$0")"

main() {
    local BLOCK_HASH
    local SQL
    local BLOCKCHAIN_STATE_ID
    local CONSENSUS_STATE_ID
    local HEIGHT

    # Loop through all JSON (block) files in the current directory
    for file in *.json
    do
        if [ -f "$file" ]; then
            # Extract the part after the second hyphen and before the .json extension
            BLOCK_HASH=$(echo "$file" | cut -d'-' -f3 | cut -d'.' -f1)
            HEIGHT=$(jq -r '.protocol_state.body.consensus_state.blockchain_length' "$file")
            echo "Processing block $BLOCK_HASH at height $HEIGHT"

            # accounts
            SQL=$(jq -r '.. | strings | select(test("^B62") and length == 55)' "$file" | sort -u | while read -r account; do
                echo "INSERT IGNORE INTO accounts (id) VALUES ('$account');"
            done)
            dolt sql --query "$SQL"

            # protocol_state
            SQL=$(jq --arg BLOCK_HASH "$BLOCK_HASH" -r '.protocol_state | "INSERT INTO protocol_state (block_hash, previous_state_hash, genesis_state_hash, blockchain_length, min_window_density, total_currency, global_slot_since_genesis, has_ancestor_in_same_checkpoint_window, block_stake_winner, block_creator, coinbase_receiver, supercharge_coinbase) VALUES (\"\($BLOCK_HASH)\", \"\(.previous_state_hash)\", \"\(.body.genesis_state_hash)\", \(.body.consensus_state.blockchain_length), \(.body.consensus_state.min_window_density), \(.body.consensus_state.total_currency), \(.body.consensus_state.global_slot_since_genesis), \(.body.consensus_state.has_ancestor_in_same_checkpoint_window), \"\(.body.consensus_state.block_stake_winner)\", \"\(.body.consensus_state.block_creator)\", \"\(.body.consensus_state.coinbase_receiver)\", \(.body.consensus_state.supercharge_coinbase));"' "$file")
            dolt sql --query "$SQL"

            # blockchain_state
            BLOCKCHAIN_STATE_ID=$(dolt sql --result-format json --query "SELECT UUID() AS id;" | jq -r '.rows[0].id')
            SQL=$(jq --arg BLOCKCHAIN_STATE_ID "$BLOCKCHAIN_STATE_ID" -r '.protocol_state.body.blockchain_state | "INSERT INTO blockchain_state (id, snarked_ledger_hash, genesis_ledger_hash, snarked_next_available_token, timestamp) VALUES (\"\($BLOCKCHAIN_STATE_ID)\", \"\(.snarked_ledger_hash)\", \"\(.genesis_ledger_hash)\", \(.snarked_next_available_token), \(.timestamp));"' "$file")
            dolt sql --query "$SQL"

            # consensus_state
            CONSENSUS_STATE_ID=$(dolt sql --result-format json --query "SELECT UUID() AS id;" | jq -r '.rows[0].id')
            SQL=$(jq --arg BLOCK_HASH "$BLOCK_HASH" --arg CONSENSUS_STATE_ID "$CONSENSUS_STATE_ID" -r '.protocol_state.body.consensus_state | "INSERT INTO consensus_state (id, block_hash, epoch_count, curr_global_slot_slot_number, curr_global_slot_slots_per_epoch) VALUES (\"\($CONSENSUS_STATE_ID)\", \"\($BLOCK_HASH)\", \(.epoch_count), \(.curr_global_slot.slot_number), \(.curr_global_slot.slots_per_epoch));"' "$file")
            dolt sql --query "$SQL"

            # staged_ledger_hash
            SQL=$(jq --arg BLOCKCHAIN_STATE_ID "$BLOCKCHAIN_STATE_ID" -r '.protocol_state.body.blockchain_state.staged_ledger_hash | "INSERT INTO staged_ledger_hash (blockchain_state_id, non_snark_ledger_hash, non_snark_aux_hash, non_snark_pending_coinbase_aux, pending_coinbase_hash) VALUES (\"\($BLOCKCHAIN_STATE_ID)\", \"\(.non_snark.ledger_hash)\", \"\(.non_snark.aux_hash)\", \"\(.non_snark.pending_coinbase_aux)\", \"\(.pending_coinbase_hash)\");"' "$file")
            dolt sql --query "$SQL"

            # epoch_data
            SQL=$(jq --arg CONSENSUS_STATE_ID "$CONSENSUS_STATE_ID" -r '.protocol_state.body.consensus_state | "INSERT INTO epoch_data (consensus_state_id, type, ledger_hash, total_currency, seed, start_checkpoint, lock_checkpoint, epoch_length) VALUES (\"\($CONSENSUS_STATE_ID)\", \"staking\", \"\(.staking_epoch_data.ledger.hash)\", \(.staking_epoch_data.ledger.total_currency), \"\(.staking_epoch_data.seed)\", \"\(.staking_epoch_data.start_checkpoint)\", \"\(.staking_epoch_data.lock_checkpoint)\", \(.staking_epoch_data.epoch_length));"' "$file")
            dolt sql --query "$SQL"
            SQL=$(jq --arg CONSENSUS_STATE_ID "$CONSENSUS_STATE_ID" -r '.protocol_state.body.consensus_state | "INSERT INTO epoch_data (consensus_state_id, type, ledger_hash, total_currency, seed, start_checkpoint, lock_checkpoint, epoch_length) VALUES (\"\($CONSENSUS_STATE_ID)\", \"next\", \"\(.next_epoch_data.ledger.hash)\", \(.next_epoch_data.ledger.total_currency), \"\(.next_epoch_data.seed)\", \"\(.next_epoch_data.start_checkpoint)\", \"\(.next_epoch_data.lock_checkpoint)\", \(.next_epoch_data.epoch_length));"' "$file")
            dolt sql --query "$SQL"

            # sub_window_densities
            SQL=$(jq --arg CONSENSUS_STATE_ID "$CONSENSUS_STATE_ID" -r '.protocol_state.body.consensus_state.sub_window_densities[] | "INSERT INTO sub_window_densities (consensus_state_id, density) VALUES (\"\($CONSENSUS_STATE_ID)\", \(.));"' "$file")
            dolt sql --query "$SQL"

            # constants
            SQL=$(jq --arg BLOCK_HASH "$BLOCK_HASH" -r '.protocol_state.body.constants | "INSERT INTO constants (k, block_hash, slots_per_epoch, slots_per_sub_window, delta, genesis_state_timestamp) VALUES (\(.k), \"\($BLOCK_HASH)\", \(.slots_per_epoch), \(.slots_per_sub_window), \(.delta), \(.genesis_state_timestamp));"' "$file")
            dolt sql --query "$SQL"

            # commands, command_status
            SQL=$(jq -r '.staged_ledger_diff.diff[0].commands[] |
            "INSERT INTO commands (fee, fee_token, fee_payer_pk, nonce, valid_until, memo, source_pk, receiver_pk, token_id, amount, signer, signature) VALUES (\(.data[1].payload.common.fee), \"\(.data[1].payload.common.fee_token)\", \"\(.data[1].payload.common.fee_payer_pk)\", \(.data[1].payload.common.nonce), \"\(.data[1].payload.common.valid_until)\", \"\(.data[1].payload.common.memo)\", \"\(.data[1].payload.body[1].source_pk)\", \"\(.data[1].payload.body[1].receiver_pk)\", \"\(.data[1].payload.body[1].token_id)\", \(.data[1].payload.body[1].amount), \"\(.data[1].signer)\", \"\(.data[1].signature)\");"' "$file")
            dolt sql --query "$SQL"

            SQL=$(jq -r '.staged_ledger_diff.diff[0].commands[] |
            "INSERT INTO command_status (status, fee_payer_account_creation_fee_paid, receiver_account_creation_fee_paid, created_token, fee_payer_balance, source_balance, receiver_balance) VALUES (\"\(.status[0])\", \(if .status[1].fee_payer_account_creation_fee_paid == null then "NULL" else .status[1].fee_payer_account_creation_fee_paid end), \(if .status[1].receiver_account_creation_fee_paid == null then "NULL" else .status[1].receiver_account_creation_fee_paid end), \(if .status[1].created_token == null then "NULL" else "\"\(.status[1].created_token)\""end), \(.status[2].fee_payer_balance), \(.status[2].source_balance), \(.status[2].receiver_balance));"' "$file")
            dolt sql --query "$SQL"

            # coinbase and fee_transfer
            SQL=$(jq -r '.staged_ledger_diff.diff[0].internal_command_balances[] | if .[0] == "Coinbase" then "INSERT INTO coinbase (type, receiver_balance) VALUES (\"Coinbase\", \(.[1].coinbase_receiver_balance));" elif .[0] == "Fee_transfer" then "INSERT INTO fee_transfer (receiver1_balance, receiver2_balance) VALUES (\(.[1].receiver1_balance), \(if .[1].receiver2_balance == null then "NULL" else .[1].receiver2_balance end));" else empty end' "$file")
            dolt sql --query "$SQL"

            dolt commit --all --message "Added block $BLOCK_HASH at height $HEIGHT"
        fi
    done

    # If no .json files were found, the loop won't execute, so we check here
    if [ ! "$(ls ./*glob*.json 2>/dev/null)" ]; then
        echo "No JSON (block) files found in the current directory."
    fi
}

main "$@"
