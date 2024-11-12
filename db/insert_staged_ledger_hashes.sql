INSERT INTO staged_ledger_hashes (
    block_hash,
    non_snark_ledger_hash,
    non_snark_aux_hash,
    non_snark_pending_coinbase_aux,
    pending_coinbase_hash
)
SELECT
    hash,
    json_extract_string(
        blockchain, '$.staged_ledger_hash.non_snark.ledger_hash'
    ),
    json_extract_string(blockchain, '$.staged_ledger_hash.non_snark.aux_hash'),
    json_extract_string(
        blockchain, '$.staged_ledger_hash.non_snark.pending_coinbase_aux'
    ),
    json_extract_string(
        blockchain, '$.staged_ledger_hash.pending_coinbase_hash'
    )
FROM extracted_state;
