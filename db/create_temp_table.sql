-- Create base temporary table for raw JSON data
CREATE TEMPORARY TABLE raw_blocks (
    hash VARCHAR,
    height BIGINT,
    data JSON
);
