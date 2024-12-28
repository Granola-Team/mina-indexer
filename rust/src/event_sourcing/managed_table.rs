use anyhow::Result;
use tokio_postgres::Client;

pub struct ManagedTable {
    client: Client,
    table_name: String,
    columns: Vec<String>,
}

impl ManagedTable {
    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_name(&self) -> &str {
        &self.table_name
    }

    pub fn get_column_count(&self) -> usize {
        self.columns.len()
    }

    /// Builder to start creating a table
    pub fn builder(client: Client) -> ManagedTableBuilder {
        ManagedTableBuilder {
            client,
            name: String::new(),
            columns: Vec::new(),
            preserve_table_data: false,
        }
    }

    /// Append a row into the table
    pub async fn insert(&self, values: &[&(dyn tokio_postgres::types::ToSql + Sync)]) -> Result<u64> {
        let column_names = self
            .columns
            .iter()
            .map(|col| col.split_whitespace().next().unwrap()) // Extract only the column names
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (1..=self.columns.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(", ");

        // Insert query
        let query = format!("INSERT INTO {} ({}) VALUES ({})", self.table_name, column_names, placeholders);

        match self.client.execute(&query, values).await {
            Ok(_) => Ok(1), // Successful insert
            Err(e) => Err(e.into()),
        }
    }

    /// Bulk insert rows into the table with a single SQL call
    pub async fn bulk_insert(&self, rows: &[Vec<&(dyn tokio_postgres::types::ToSql + Sync)>]) -> Result<u64> {
        if rows.is_empty() {
            return Ok(0);
        }

        // Generate column names and placeholders
        let column_names = self
            .columns
            .iter()
            .map(|col| col.split_whitespace().next().unwrap()) // Extract column names
            .collect::<Vec<_>>()
            .join(", ");
        let column_count = self.columns.len();
        let placeholder_sets: Vec<String> = (0..rows.len())
            .map(|row_index| {
                let start = row_index * column_count + 1;
                let end = start + column_count - 1;
                let placeholders = (start..=end).map(|i| format!("${}", i)).collect::<Vec<_>>();
                format!("({})", placeholders.join(", "))
            })
            .collect();

        let query = format!("INSERT INTO {} ({}) VALUES {}", self.table_name, column_names, placeholder_sets.join(", "));

        // Flatten rows into a single list of parameters
        let flattened_values: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = rows.iter().flat_map(|row| row.iter().copied()).collect();

        // Execute the query
        let result = self.client.execute(&query, &flattened_values).await?;
        Ok(result)
    }
}

/// Builder for the ManagedTable
pub struct ManagedTableBuilder {
    client: Client,
    name: String,
    columns: Vec<String>,
    preserve_table_data: bool,
}

impl ManagedTableBuilder {
    pub fn get_client(&self) -> &Client {
        &self.client
    }
}

impl ManagedTableBuilder {
    /// Set the name for the table
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Add a column to the table
    pub fn add_column(mut self, column_definition: &str) -> Self {
        self.columns.push(column_definition.to_string());
        self
    }

    pub fn preserve_table_data(mut self) -> Self {
        self.preserve_table_data = true;
        self
    }

    /// Build and initialize the table, optionally deleting rows based on root node
    pub async fn build(self, root: &Option<(u64, String)>) -> Result<ManagedTable> {
        let table_name = self.name.to_string();

        // Ensure the `height` column is specified
        if !self.columns.iter().any(|col| col.starts_with("height")) {
            panic!("The column 'height' is required but not found.");
        }

        // Drop the table
        if root.is_none() {
            if !self.preserve_table_data {
                self.drop_table(&table_name).await?;
            }
        }

        // Create the table
        self.create_table(&table_name).await?;

        // Handle root node deletion if provided
        if let Some((height, state_hash)) = root {
            if !self.preserve_table_data {
                self.handle_root_node_deletion(&table_name, height.to_owned(), state_hash).await?;
            }
        }

        // Return the ManagedTable instance
        Ok(ManagedTable {
            client: self.client,
            table_name,
            columns: self.columns,
        })
    }

    // Drop the table
    async fn drop_table(&self, table_name: &str) -> Result<()> {
        let drop_table = format!("DROP TABLE IF EXISTS {} CASCADE;", table_name);
        self.client.execute(&drop_table, &[]).await?;
        Ok(())
    }

    // Create the table
    async fn create_table(&self, table_name: &str) -> Result<()> {
        let table_query = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                entry_id BIGSERIAL,
                height BIGINT,
                {},
                PRIMARY KEY (entry_id)
            );",
            table_name,
            self.columns
                .iter()
                .filter(|c| !c.starts_with("height")) // Ensure `height` is not added here
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
                .join(",\n")
        );
        self.client.execute(&table_query, &[]).await?;
        Ok(())
    }

    // Handle root node deletion: truncate rows with the given height and state_hash
    async fn handle_root_node_deletion(&self, table_name: &str, height: u64, state_hash: &str) -> Result<()> {
        let truncate_query = format!("DELETE FROM {} WHERE height > $1 OR (height = $1 AND state_hash = $2);", table_name);

        self.client.execute(&truncate_query, &[&(height as i64), &state_hash.to_string()]).await?;

        Ok(())
    }
}

#[cfg(test)]
mod test_table_tests {
    use super::*;
    use crate::constants::POSTGRES_CONNECTION_STRING;
    use tokio_postgres::NoTls;

    #[tokio::test]
    async fn test_test_table_inserts() {
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

        // Setup the ManagedTable
        let test_table = ManagedTable::builder(client)
            .name("test_table")
            .add_column("height BIGINT") // Explicitly adding the height column
            .add_column("state_hash TEXT")
            .add_column("timestamp BIGINT")
            .build(&None) // No root, so no deletion
            .await
            .expect("Failed to buildtable");

        // Insert data with explicit height parameter
        test_table
            .insert(&[&1_i64, &"state_hash_1", &1234567890i64])
            .await
            .expect("Failed to insert log");

        test_table
            .insert(&[&2_i64, &"state_hash_2", &1234567891i64])
            .await
            .expect("Failed to insert log");

        test_table
            .insert(&[&3_i64, &"state_hash_3", &1234567892i64])
            .await
            .expect("Failed to insert log");

        // Query the table to verify inserts
        let log_query = "SELECT * FROM test_table WHERE height = $1";
        let log_rows = test_table.client.query(log_query, &[&(1_i64)]).await.expect("Failed to query test_table table");

        // Assert the correct rows are present
        assert_eq!(log_rows.len(), 1, "Expected 1 row in the table");

        let log_rows = test_table.client.query(log_query, &[&(1_i64)]).await.expect("Failed to query test_table table");
        assert_eq!(log_rows.len(), 1, "Expected 1 row in the table");

        let log_rows = test_table.client.query(log_query, &[&(2_i64)]).await.expect("Failed to query test_table table");
        assert_eq!(log_rows.len(), 1, "Expected 1 row in the table");
    }

    #[tokio::test]
    async fn test_test_table_root_deletion_with_3_heights() {
        {
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

            // Setup the ManagedTable
            let test_table = ManagedTable::builder(client)
                .name("test_table")
                .add_column("height BIGINT") // Explicitly adding the height column
                .add_column("state_hash TEXT")
                .add_column("timestamp BIGINT")
                .build(&None) // No root initially, so no deletion
                .await
                .expect("Failed to buildtable");

            // Insert 3 rows for each of 3 heights: 0, 1, and 2 (total of 9 rows)
            // Ensure the height corresponds to 1, 2, or 3
            test_table
                .insert(&[&1_i64, &"state_hash_1_1", &1234567890i64])
                .await
                .expect("Failed to insert log");

            test_table
                .insert(&[&2_i64, &"state_hash_1_2", &1234567891i64])
                .await
                .expect("Failed to insert log");

            test_table
                .insert(&[&3_i64, &"state_hash_1_3", &1234567892i64])
                .await
                .expect("Failed to insert log");

            test_table
                .insert(&[&1_i64, &"state_hash_2_1", &1234567893i64])
                .await
                .expect("Failed to insert log");

            test_table
                .insert(&[&2_i64, &"state_hash_2_2", &1234567894i64])
                .await
                .expect("Failed to insert log");

            test_table
                .insert(&[&3_i64, &"state_hash_2_3", &1234567895i64])
                .await
                .expect("Failed to insert log");

            test_table
                .insert(&[&1_i64, &"state_hash_3_1", &1234567896i64])
                .await
                .expect("Failed to insert log");

            test_table
                .insert(&[&2_i64, &"state_hash_3_2", &1234567897i64])
                .await
                .expect("Failed to insert log");

            test_table
                .insert(&[&3_i64, &"state_hash_3_3", &1234567898i64])
                .await
                .expect("Failed to insert log");
        }

        {
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
            // Set the root node to height 2 (this will delete height 2 and its children)
            let root = Some((2, "state_hash_2_2".to_string()));
            let test_table = ManagedTable::builder(client)
                .name("test_table")
                .add_column("height BIGINT")
                .add_column("state_hash TEXT")
                .add_column("timestamp BIGINT")
                .build(&root) // Set root for deletion
                .await
                .expect("Failed to rebuildtable with root");
            let log_query = "SELECT * FROM test_table WHERE height = $1";
            // Query for height 1 and ensure that rows still exist for height 1
            let log_rows = test_table
                .client
                .query(log_query, &[&(1_i64)])
                .await
                .expect("Failed to query test_table table for height 1");

            // Assert that rows for height 1 are still present
            assert_eq!(log_rows.len(), 3, "Expected 3 rows in the table for height 1");

            // Query for height 3 and ensure that rows still exist for height 3
            let log_rows = test_table
                .client
                .query(log_query, &[&(3_i64)])
                .await
                .expect("Failed to query test_table table for height 3");

            // Assert that rows for height 3 are still present
            assert_eq!(log_rows.len(), 0, "Expected 0 rows in the table for height 3");

            // Query for height 2 and check that the state_hash rows are deleted
            let log_query = "SELECT * FROM test_table WHERE height = $1";
            let deleted_row = test_table
                .client
                .query(log_query, &[&(2_i64)])
                .await
                .expect("Failed to query test_table table for specific state_hash");

            // Ensure no rows exist for the deleted state_hash at height 2
            assert_eq!(deleted_row.len(), 2, "Expected 2 rows at height 2");
        }
    }

    #[tokio::test]
    async fn test_managed_table_bulk_insert() {
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

        // Setup the ManagedTable
        let test_table = ManagedTable::builder(client)
            .name("test_table")
            .add_column("height BIGINT")
            .add_column("state_hash TEXT")
            .add_column("timestamp BIGINT")
            .build(&None)
            .await
            .expect("Failed to build table");

        // Prepare bulk values
        let bulk_values = vec![
            vec![&1_i64 as &(dyn tokio_postgres::types::ToSql + Sync), &"state_hash_1", &1234567890_i64],
            vec![&2_i64, &"state_hash_2", &1234567891_i64],
            vec![&3_i64, &"state_hash_3", &1234567892_i64],
        ];

        // Execute bulk insert
        let rows_inserted = test_table.bulk_insert(&bulk_values).await.expect("Failed to bulk insert logs");

        // Verify the number of rows inserted
        assert_eq!(rows_inserted, 3, "Expected 3 rows to be inserted");

        // Query the table to verify the inserted rows
        let query = "SELECT * FROM test_table ORDER BY height";
        let rows = test_table.client.query(query, &[]).await.expect("Failed to query test_table");

        assert_eq!(rows.len(), 3, "Expected 3 rows in the table");
        let heights: Vec<i64> = rows.iter().map(|row| row.get("height")).collect();
        assert_eq!(heights, vec![1, 2, 3], "Heights do not match expected values");
    }

    #[tokio::test]
    async fn test_preserve_table_data() {
        // 1) Connect to the database
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to the database");
        // Spawn the connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // 2) First build with `preserve_table_data = false` (the default). Then insert some rows.
        let table_name = "test_preserve_table_data";
        {
            let managed_table = ManagedTable::builder(client)
                .name(table_name)
                .add_column("height BIGINT")
                .add_column("some_value TEXT")
                // not calling .preserve_table_data(), so it's `false` by default
                .build(&None) // no root pruning
                .await
                .expect("Failed to build table for the first time");

            // Insert a couple of rows
            managed_table.insert(&[&1_i64, &"some_value_1"]).await.expect("Failed to insert row #1");
            managed_table.insert(&[&2_i64, &"some_value_2"]).await.expect("Failed to insert row #2");

            // Verify we have 2 rows
            let rows = managed_table
                .get_client()
                .query(&format!("SELECT * FROM {} ORDER BY height", table_name), &[])
                .await
                .expect("Failed to query test table");
            assert_eq!(rows.len(), 2, "Expected 2 rows in the first pass");
        } // The `managed_table` goes out of scope here, but table remains in DB.

        // 3) Build again, but now call `.preserve_table_data()`. We expect the table is NOT dropped or truncated, so the prior rows remain.
        {
            let (client2, connection2) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
                .await
                .expect("Failed to connect to the database for second pass");
            tokio::spawn(async move {
                if let Err(e) = connection2.await {
                    eprintln!("Connection2 error: {}", e);
                }
            });

            let managed_table_2 = ManagedTable::builder(client2)
                .name(table_name)
                .add_column("height BIGINT")
                .add_column("some_value TEXT")
                .preserve_table_data() // <--- crucial difference
                .build(&None)
                .await
                .expect("Failed to build table for the second pass with preserve_table_data");

            // Query to see if old rows remain
            let rows = managed_table_2
                .get_client()
                .query(&format!("SELECT * FROM {} ORDER BY height", table_name), &[])
                .await
                .expect("Failed to query test table after preserving data");
            assert_eq!(rows.len(), 2, "Expected old rows to remain with preserve_table_data = true");

            // Insert an additional row now
            managed_table_2.insert(&[&3_i64, &"new_value_3"]).await.expect("Failed to insert row #3");

            let rows = managed_table_2
                .get_client()
                .query(&format!("SELECT * FROM {} ORDER BY height", table_name), &[])
                .await
                .expect("Failed to query test table after new insert");
            assert_eq!(rows.len(), 3, "Expected the table to contain the original 2 rows plus the new one");
        }
    }
}
