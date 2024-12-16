use anyhow::Result;
use duckdb::Connection;

pub(crate) mod actors;
pub(crate) mod blockchain_tree;
pub mod blocks;
pub mod constants;
pub mod event_sourcing;
pub mod files;
pub mod staking;
pub mod utility;

const DB_FILE: &str = "mina.db";

pub fn get_db_connection() -> Result<Connection, duckdb::Error> {
    let db = Connection::open(DB_FILE)?;

    db.execute_batch(
        "
            SET maximum_object_size='512MB';
            SET memory_limit='16GB';
        ",
    )?;

    Ok(db)
}

pub fn check_or_create_db_schema() -> Result<(), duckdb::Error> {
    let db = get_db_connection()?;

    // Create final schema
    let schema = include_str!("../../db/schema.sql");
    db.execute_batch(schema)?;

    Ok(())
}
