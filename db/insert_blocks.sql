INSERT INTO blocks (
    hash,
    previous_hash,
    genesis_hash,
    blockchain_length,
    epoch,
    global_slot_since_genesis,
    scheduled_time,
    total_currency,
    stake_winner,
    creator,
    coinbase_target,
    supercharge_coinbase,
    last_vrf_output,
    min_window_density,
    has_ancestor_in_same_checkpoint_window
)
SELECT
    hash,
    previous_hash,
    json_extract_string(body, '$.genesis_state_hash'),
    height,
    CAST(json_extract(consensus, '$.epoch_count') AS BIGINT),
    CAST(json_extract(consensus, '$.global_slot_since_genesis') AS BIGINT),
    scheduled_time,
    CAST(json_extract(consensus, '$.total_currency') AS BIGINT),
    json_extract_string(consensus, '$.block_stake_winner'),
    json_extract_string(consensus, '$.block_creator'),
    json_extract_string(consensus, '$.coinbase_receiver'),
    CAST(json_extract(consensus, '$.supercharge_coinbase') AS BOOLEAN),
    json_extract_string(consensus, '$.last_vrf_output'),
    CAST(json_extract(consensus, '$.min_window_density') AS BIGINT),
    CAST(json_extract(consensus, '$.has_ancestor_in_same_checkpoint_window') AS BOOLEAN)
FROM extracted_state;
