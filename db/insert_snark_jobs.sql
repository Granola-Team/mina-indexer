INSERT INTO snark_jobs (id, block_hash, prover, fee)
SELECT
    nextval('snark_jobs_id_seq'),
    hash,
    json_extract_string(works, '$.prover'),
    cast(json_extract(works, '$.fee') AS DECIMAL(38, 9))
FROM temp_completed_works
WHERE works IS NOT NULL;