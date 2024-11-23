use anyhow::Result;
use tokio_postgres::Client;

pub struct DbLogger {
    pub client: Client,
    table_name: String,
    columns: Vec<String>,
}

impl DbLogger {
    /// Builder to start creating a DbLogger
    pub fn builder(client: Client) -> DbLoggerBuilder {
        DbLoggerBuilder {
            client,
            name: String::new(),
            columns: Vec::new(),
        }
    }

    /// Insert a row into the table
    pub async fn insert(&self, values: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        let column_names = self
            .columns
            .iter()
            .map(|col| col.split_whitespace().next().unwrap()) // Extract only the column names
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (1..=self.columns.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(", ");

        let query = format!("INSERT INTO {} ({}) VALUES ({})", self.table_name, column_names, placeholders);

        self.client.execute(&query, values).await.map_err(|e| {
            eprintln!("Failed to insert row into {}: {:?}", self.table_name, e);
            e.into()
        })
    }
}

/// Builder for the DbLogger
pub struct DbLoggerBuilder {
    client: Client,
    name: String,
    columns: Vec<String>,
}

impl DbLoggerBuilder {
    /// Set the name for the logger
    /// The table will be `{name}_dirty` and the view will be `{name}`
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Add a column to the table
    pub fn add_column(mut self, column_definition: &str) -> Self {
        self.columns.push(column_definition.to_string());
        self
    }
    /// Build and initialize the table and view, dropping any existing table, view, and sequence first
    pub async fn build(self, drop: bool) -> Result<DbLogger> {
        let table_name = format!("{}_dirty", self.name);
        let view_name = self.name.clone();
        let sequence_name = format!("{}_entry_id_seq", table_name);

        if drop {
            // Drop the existing table, view, and sequence
            let drop_table_query = format!("DROP TABLE IF EXISTS {} CASCADE;", table_name);
            let drop_view_query = format!("DROP VIEW IF EXISTS {};", view_name);
            let drop_sequence_query = format!("DROP SEQUENCE IF EXISTS {} CASCADE;", sequence_name);

            self.client.execute(&drop_table_query, &[]).await?;
            self.client.execute(&drop_view_query, &[]).await?;
            self.client.execute(&drop_sequence_query, &[]).await?;
        }

        // Create the sequence
        let create_sequence_query = format!("CREATE SEQUENCE IF NOT EXISTS {};", sequence_name);
        self.client.execute(&create_sequence_query, &[]).await?;

        // Create the table
        let table_query = format!(
            "CREATE TABLE {} (
                entry_id BIGINT DEFAULT nextval('{}') PRIMARY KEY,
                {}
            );",
            table_name,
            sequence_name,
            self.columns.join(",\n")
        );

        self.client.execute(&table_query, &[]).await?;

        // Create the view
        let distinct_columns = self
            .columns
            .iter()
            .map(|col| col.split_whitespace().next().unwrap()) // Extract column names
            .collect::<Vec<_>>()
            .join(", ");

        let view_query = format!(
            "CREATE OR REPLACE VIEW {} AS
            SELECT DISTINCT ON ({}) *
            FROM {}
            ORDER BY {}, entry_id DESC;",
            view_name, distinct_columns, table_name, distinct_columns
        );

        self.client.execute(&view_query, &[]).await?;

        Ok(DbLogger {
            client: self.client,
            table_name,
            columns: self.columns,
        })
    }
}

#[cfg(test)]
mod db_logger_tests {
    use super::*;
    use crate::constants::POSTGRES_CONNECTION_STRING;
    use tokio_postgres::NoTls;

    #[tokio::test]
    async fn test_db_logger_inserts_and_view_distinct() {
        // Connect to the database
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to the database");

        // Spawn the connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // Setup the logger with fewer columns
        let logger = DbLogger::builder(client)
            .name("log")
            .add_column("height BIGINT")
            .add_column("state_hash TEXT")
            .add_column("timestamp BIGINT")
            .build(true)
            .await
            .expect("Failed to build logger");

        logger.insert(&[&1i64, &"state_hash_1", &1234567890i64]).await.expect("Failed to insert log");

        logger.insert(&[&1i64, &"state_hash_1", &1234567890i64]).await.expect("Failed to insert log");

        logger.insert(&[&1i64, &"state_hash_1", &1234567890i64]).await.expect("Failed to insert log");

        // Query the raw log table
        let log_query = "SELECT * FROM log_dirty WHERE height = $1 AND state_hash = $2";
        let log_rows = logger
            .client
            .query(log_query, &[&(1_i64), &"state_hash_1"])
            .await
            .expect("Failed to query log table");

        // Assert all rows are present in the log
        assert_eq!(log_rows.len(), 3, "Expected 3 rows in the log table");

        // Query the view
        let view_query = "SELECT * FROM log WHERE height = $1";
        let view_rows = logger.client.query(view_query, &[&(1_i64)]).await.expect("Failed to query view");

        // Assert only one row is present in the view
        assert_eq!(view_rows.len(), 1, "Expected 1 row in the view");

        // Assert the row in the view corresponds to the latest `entry_id`
        let latest_row = &view_rows[0];
        let latest_timestamp: i64 = latest_row.get("timestamp");
        assert_eq!(latest_timestamp, 1234567890i64,);
    }
}
