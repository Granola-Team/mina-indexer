WITH epoch_types(path, label) AS (
    VALUES
        ('staking_epoch_data', 'staking'),
        ('next_epoch_data', 'next')
),
epoch_data_source AS (
    SELECT
        e.hash,
        json_extract_string(e.consensus, '$.' || t.path || '.ledger.hash') as ledger_hash,
        CAST(json_extract(e.consensus, '$.' || t.path || '.ledger.total_currency') AS BIGINT) as total_currency,
        json_extract_string(e.consensus, '$.' || t.path || '.seed') as seed,
        json_extract_string(e.consensus, '$.' || t.path || '.start_checkpoint') as start_checkpoint,
        json_extract_string(e.consensus, '$.' || t.path || '.lock_checkpoint') as lock_checkpoint,
        CAST(json_extract(e.consensus, '$.' || t.path || '.epoch_length') AS BIGINT) as epoch_length,
        t.label as type
    FROM extracted_state e
    CROSS JOIN epoch_types t
)
INSERT INTO epoch_data (
    block_hash,
    ledger_hash,
    total_currency,
    seed,
    start_checkpoint,
    lock_checkpoint,
    epoch_length,
    type
)
SELECT * FROM epoch_data_source;
