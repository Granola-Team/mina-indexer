use crate::{
    files::{extract_digits_from_file_name, extract_hash_from_file_name, get_file_paths},
    get_db_connection,
};
use anyhow::Result;
use duckdb::{params_from_iter, OptionalExt};
use tracing::info;

pub fn run(dir: &str) -> Result<()> {
    info!("Processing files in: {}", dir);
    let paths = get_file_paths(dir)?;
    let db = get_db_connection()?;

    for path in &paths {
        let file_path = path.to_str().unwrap();
        let hash = extract_hash_from_file_name(path);
        let number = extract_digits_from_file_name(path);

        // Load JSON file into staging table with increased limits
        db.execute_batch(&format!(
            "DELETE FROM raw_json;
                     COPY raw_json (data) FROM '{}' (FORMAT JSON, AUTO_DETECT FALSE);
                     UPDATE raw_json SET
                        file_hash = '{}',
                        file_number = {}
                     WHERE file_hash IS NULL;",
            file_path, hash, number
        ))?;

        // Process staking epochs
        db.execute(
            "INSERT INTO staking_epochs (hash, epoch)
                    SELECT DISTINCT file_hash, file_number
                    FROM raw_json
                    ON CONFLICT (hash, epoch) DO NOTHING",
            [],
        )?;

        // Process accounts
        db.execute_batch(
            "
                   WITH RECURSIVE
                   array_elements AS (
                       SELECT file_hash,
                              json_array(data) as items,
                              generate_series(0, json_array_length(data) - 1) as idx
                       FROM raw_json
                   )
                   INSERT INTO accounts (public_key)
                   SELECT DISTINCT json_extract_string(items[idx], '$.pk') as pk
                   FROM array_elements
                   WHERE json_extract_string(items[idx], '$.pk') IS NOT NULL
                   AND NOT EXISTS (
                       SELECT 1 FROM accounts WHERE public_key = json_extract_string(items[idx], '$.pk')
                   );

                   WITH RECURSIVE
                   array_elements AS (
                       SELECT file_hash,
                              json_array(data) as items,
                              generate_series(0, json_array_length(data) - 1) as idx
                       FROM raw_json
                   )
                   INSERT INTO accounts (public_key)
                   SELECT DISTINCT json_extract_string(items[idx], '$.delegate') as delegate
                   FROM array_elements
                   WHERE json_extract_string(items[idx], '$.delegate') IS NOT NULL
                   AND NOT EXISTS (
                       SELECT 1 FROM accounts WHERE public_key = json_extract_string(items[idx], '$.delegate')
                   );
               ",
        )?;

        // Process staking ledgers
        db.execute_batch(
            "
                   WITH RECURSIVE
                   array_elements AS (
                       SELECT file_hash,
                              file_number,
                              json_array(data) as items,
                              generate_series(0, json_array_length(data) - 1) as idx
                       FROM raw_json
                   )
                   INSERT INTO staking_ledgers (
                       epoch_hash, epoch_number, source, balance, target, token,
                       nonce, receipt_chain_hash, voting_for
                   )
                   SELECT
                       file_hash,
                       file_number,
                       json_extract_string(items[idx], '$.pk'),
                       CAST(json_extract_string(items[idx], '$.balance') AS DECIMAL),
                       COALESCE(
                           json_extract_string(items[idx], '$.delegate'),
                           json_extract_string(items[idx], '$.pk')
                       ),
                       COALESCE(CAST(json_extract_string(items[idx], '$.token') AS BIGINT), 0),
                       COALESCE(CAST(json_extract_string(items[idx], '$.nonce') AS BIGINT), 0),
                       COALESCE(json_extract_string(items[idx], '$.receipt_chain_hash'), ''),
                       COALESCE(json_extract_string(items[idx], '$.voting_for'), '')
                   FROM array_elements
                   WHERE json_extract_string(items[idx], '$.pk') IS NOT NULL
                   RETURNING id;
               ",
        )?;

        // Process timing data
        db.execute_batch(
            "
                   WITH RECURSIVE
                   array_elements AS (
                       SELECT l.id,
                              json_array(r.data) as items,
                              generate_series(0, json_array_length(r.data) - 1) as idx
                       FROM staking_ledgers l
                       JOIN raw_json r ON l.epoch_hash = r.file_hash
                   )
                   INSERT INTO staking_timing (
                       ledger_id,
                       initial_minimum_balance,
                       cliff_time,
                       cliff_amount,
                       vesting_period,
                       vesting_increment
                   )
                   SELECT
                       id,
                       CAST(json_extract_string(items[idx], '$.timing.initial_minimum_balance') AS DECIMAL),
                       CAST(json_extract_string(items[idx], '$.timing.cliff_time') AS BIGINT),
                       CAST(json_extract_string(items[idx], '$.timing.cliff_amount') AS DECIMAL),
                       CAST(json_extract_string(items[idx], '$.timing.vesting_period') AS BIGINT),
                       CAST(json_extract_string(items[idx], '$.timing.vesting_increment') AS DECIMAL)
                   FROM array_elements
                   WHERE json_extract_string(items[idx], '$.timing') IS NOT NULL;
               ",
        )?;
    }

    // Cleanup
    db.execute_batch("DROP TABLE raw_json;")?;

    Ok(())
}
