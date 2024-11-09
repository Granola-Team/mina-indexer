use crate::{
    files::{extract_digits_from_file_name, extract_hash_from_file_name, get_file_paths},
    get_db_connection,
};
use anyhow::Result;
use tracing::{error, info};

pub fn run(dir: &str) -> Result<()> {
    info!("Processing files in: {}", dir);
    let paths = get_file_paths(dir)?;
    let db = get_db_connection()?;

    for path in &paths {
        let file_path = path.to_str().unwrap();
        let ledger_hash = extract_hash_from_file_name(path);
        let epoch = extract_digits_from_file_name(path);

        info!("Processing file: {}", file_path);

        // First, load the JSON data into ledgers table
        let sql = format!(
            r#"WITH
                    json_data AS (
                        SELECT * FROM read_json_objects('{}')
                    )
                    INSERT INTO ledgers (
                        pk, balance, delegate, token, nonce, receipt_chain_hash,
                        voting_for, timing, token_symbol, ledger_hash, epoch
                    )
                    SELECT
                        json_extract_string(json, '$.pk'),
                        CAST(json_extract_string(json, '$.balance') AS DECIMAL(20,10)),
                        json_extract_string(json, '$.delegate'),
                        json_extract_string(json, '$.token'),
                        CAST(json_extract_string(json, '$.nonce') AS BIGINT),
                        json_extract_string(json, '$.receipt_chain_hash'),
                        json_extract_string(json, '$.voting_for'),
                        CASE
                            WHEN json_extract_string(json, '$.timing') IS NOT NULL
                            THEN json_extract_string(json, '$.timing')::JSON
                            END,
                        json_extract_string(json, '$.token_symbol'),
                        '{}',
                        {}
                    FROM json_data;"#,
            file_path, ledger_hash, epoch
        );

        match db.execute_batch(&sql) {
            Ok(_) => {
                info!("Successfully updated ledger_hash {ledger_hash} and epoch {epoch}")
            }
            Err(e) => {
                error!("Error copying data from {}: {}", file_path, e);
                return Err(e.into());
            }
        }

        // Process accounts
        db.execute_batch(
            "
            INSERT INTO accounts (public_key)
            SELECT DISTINCT val FROM (
                SELECT pk as val FROM ledgers
                UNION
                SELECT delegate as val FROM ledgers WHERE delegate IS NOT NULL
            ) unique_keys
            WHERE NOT EXISTS (
                SELECT 1 FROM accounts WHERE public_key = unique_keys.val
            );
            ",
        )?;

        // Process staking epochs
        db.execute_batch(&format!(
            "
                    INSERT INTO staking_epochs (hash, epoch)
                    SELECT '{}', {}
                    WHERE NOT EXISTS (
                        SELECT 1 FROM staking_epochs
                        WHERE hash = '{}' AND epoch = {}
                    )
                    RETURNING id;
                    ",
            ledger_hash, epoch, ledger_hash, epoch
        ))?;

        // Process staking ledgers with new relationship
        db.execute_batch(
            "
                    INSERT INTO staking_ledgers (
                        staking_epoch_id, source, balance, target, token,
                        nonce, receipt_chain_hash, voting_for
                    )
                    SELECT
                        se.id,
                        l.pk,
                        l.balance,
                        COALESCE(l.delegate, l.pk),
                        COALESCE(l.token, '0'),
                        l.nonce,
                        l.receipt_chain_hash,
                        l.voting_for
                    FROM ledgers l
                    JOIN staking_epochs se ON se.hash = l.ledger_hash AND se.epoch = l.epoch
                    WHERE NOT EXISTS (
                        SELECT 1 FROM staking_ledgers sl
                        WHERE sl.staking_epoch_id = se.id
                        AND sl.source = l.pk
                    );
                    ",
        )?;

        // Process timing data
        db.execute_batch(
            "
                    INSERT INTO staking_timing (
                        ledger_id,
                        initial_minimum_balance,
                        cliff_time,
                        cliff_amount,
                        vesting_period,
                        vesting_increment
                    )
                    SELECT
                        sl.id,
                        CAST(l.timing->>'initial_minimum_balance' AS DECIMAL),
                        CAST(l.timing->>'cliff_time' AS BIGINT),
                        CAST(l.timing->>'cliff_amount' AS DECIMAL),
                        CAST(l.timing->>'vesting_period' AS BIGINT),
                        CAST(l.timing->>'vesting_increment' AS DECIMAL)
                    FROM staking_ledgers sl
                    JOIN staking_epochs se ON sl.staking_epoch_id = se.id
                    JOIN ledgers l ON sl.source = l.pk
                        AND se.hash = l.ledger_hash
                        AND se.epoch = l.epoch
                    WHERE l.timing IS NOT NULL
                    AND NOT EXISTS (
                        SELECT 1 FROM staking_timing
                        WHERE ledger_id = sl.id
                    );
                    ",
        )?;

        // Clear ledgers table for next file
        db.execute_batch("DELETE FROM ledgers;")?;
    }

    Ok(())
}
