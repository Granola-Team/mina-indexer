use anyhow::Result;
use tokio_postgres::Client;

/// A "key/value" store with user-defined columns, built on top of PostgreSQL.
///
/// Example usage:
///  1) `ManagedStore::builder(client) .name("my_store") .add_numeric_column("counter")    // defaults to 0 .add_text_column("comment")       // defaults to ''
///     .build(...)`
///  2) `store.upsert("my_key", &[("counter", &123i64), ("comment", &"Hello!")]).await?;`
pub struct ManagedStore {
    client: Client,
    store_name: String,
    columns: Vec<StoreColumn>,
}

/// Each column has: name, type, and default value.
/// For simplicity, we show two primary forms: numeric and text.
#[derive(Debug)]
pub enum StoreColumnType {
    Numeric, // e.g. BIGINT default 0
    Text,    // e.g. TEXT default ''
}

/// A small struct to hold the name/type for each column.
#[derive(Debug)]
pub struct StoreColumn {
    pub name: String,
    pub col_type: StoreColumnType,
    pub default_value: String, // e.g. "0" for numeric, "''" for text
}

/// The builder to configure `ManagedStore`.
pub struct ManagedStoreBuilder {
    client: Client,
    store_name: String,
    columns: Vec<StoreColumn>,
    preserve_data: bool, // If true, do not drop the table on build
}

impl ManagedStore {
    /// Returns a reference to the underlying `Client`.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Returns the store name.
    pub fn name(&self) -> &str {
        &self.store_name
    }

    /// Returns the columns for debugging or introspection.
    pub fn columns(&self) -> &[StoreColumn] {
        &self.columns
    }

    /// Builder entry-point, akin to `ManagedTable::builder(...)`.
    pub fn builder(client: Client) -> ManagedStoreBuilder {
        ManagedStoreBuilder {
            client,
            store_name: String::new(),
            columns: Vec::new(),
            preserve_data: false,
        }
    }

    /// Upsert a set of column-values for the given `key`.
    ///
    /// `pairs` should contain `(column_name, value_as_ToSql)`.
    ///
    /// The store enforces a single row per `key`.
    /// If the row already exists, we update the given columns.
    /// If it doesn't exist, we insert the row and set all unspecified columns
    /// to their default values.
    pub async fn upsert(&self, key: &str, pairs: &[(&str, &(dyn tokio_postgres::types::ToSql + Sync))]) -> Result<u64> {
        // We'll build two sets:
        //   1) columns_to_update => "col1 = EXCLUDED.col1, col2 = EXCLUDED.col2, ..."
        //   2) the placeholders for inserting a new row => (key, col1, col2, ...)
        //
        // But we have to fill *all* columns in the INSERT, using default values for those not in `pairs`.

        // Build a map {col_name -> param_index} for those being updated
        let mut update_cols = Vec::new();
        let mut insert_cols = Vec::new();
        let mut insert_placeholders = Vec::new();
        let mut all_values = Vec::new();

        // Always the first column is `key TEXT PRIMARY KEY`.
        // We'll do "key" first in the insertion list.
        insert_cols.push("key".to_string());
        insert_placeholders.push("$1".to_string());
        all_values.push(&key as &(dyn tokio_postgres::types::ToSql + Sync));

        // Next, we must ensure we supply *every* column's value or default.
        // We'll keep track of which columns are explicitly in `pairs`.
        let pairs_map: std::collections::HashMap<&str, usize> = pairs.iter().enumerate().map(|(i, (col, _val))| (*col, i)).collect();

        // We already used param #1 for the `key`.
        let mut next_param_index = 2;
        for col_def in &self.columns {
            if col_def.name == "key" {
                continue; // skip the "key" column because we handled it above
            }
            match pairs_map.get(col_def.name.as_str()) {
                Some(&pairs_idx) => {
                    // This column is provided by the caller
                    let placeholder = format!("${}", next_param_index);
                    next_param_index += 1;
                    insert_cols.push(col_def.name.clone());
                    insert_placeholders.push(placeholder.clone());
                    all_values.push(pairs[pairs_idx].1);

                    // For the ON CONFLICT part, we do "col = EXCLUDED.col"
                    update_cols.push(format!("{} = EXCLUDED.{}", col_def.name, col_def.name));
                }
                None => {
                    // Not provided => use the default in the INSERT
                    // e.g. "DEFAULT"
                    insert_cols.push(col_def.name.clone());
                    insert_placeholders.push(col_def.default_value.clone());
                    // For param array, we do NOT push anything because it's literal
                    // But to keep them all consistent, we'll not do param placeholders. We can inline the literal.
                    // For updating, we do *not* add them to the update clause
                    // because we only want to update columns that were explicitly set,
                    // or we risk overwriting previously set columns with the default each time.
                }
            }
        }

        // Build the INSERT statement
        // E.g.:
        //   INSERT INTO store_name (key, colA, colB, ...)
        //   VALUES ($1, $2, <default>, ...)
        //   ON CONFLICT (key)
        //   DO UPDATE SET colA = EXCLUDED.colA, colB = EXCLUDED.colB, ...
        //
        // where colX not in pairs is left out of the DO UPDATE clause
        // to preserve existing data for that column.

        let insert_cols_str = insert_cols.join(", ");
        let insert_placeholders_str = insert_placeholders.join(", ");

        let conflict_update_str = if update_cols.is_empty() {
            // If no columns are being updated, do nothing
            String::from("DO NOTHING")
        } else {
            format!("DO UPDATE SET {}", update_cols.join(", "))
        };

        // e.g.:
        //   INSERT INTO my_store (key, colA, colB)
        //   VALUES ($1, $2, 0)
        //   ON CONFLICT (key) DO UPDATE SET colA = EXCLUDED.colA
        let stmt = format!(
            "INSERT INTO {tbl} ({cols}) VALUES ({vals}) ON CONFLICT (key) {conf};",
            tbl = self.store_name,
            cols = insert_cols_str,
            vals = insert_placeholders_str,
            conf = conflict_update_str,
        );

        let rows_affected = self.client.execute(&stmt, &all_values).await?;
        Ok(rows_affected)
    }
}

impl ManagedStoreBuilder {
    /// Set the store name (this will be the table name)
    pub fn name(mut self, store_name: &str) -> Self {
        self.store_name = store_name.to_string();
        self
    }

    /// Indicate not to drop the store if it already exists
    pub fn preserve_data(mut self) -> Self {
        self.preserve_data = true;
        self
    }

    /// Add a numeric column, which is stored as `BIGINT DEFAULT 0`.
    pub fn add_numeric_column(mut self, col_name: &str) -> Self {
        let col = StoreColumn {
            name: col_name.to_string(),
            col_type: StoreColumnType::Numeric,
            default_value: "DEFAULT".to_string(), // We'll inline "DEFAULT" in the insert placeholders
        };
        self.columns.push(col);
        self
    }

    /// Add a text column, which is stored as `TEXT DEFAULT ''` by default.
    pub fn add_text_column(mut self, col_name: &str) -> Self {
        let col = StoreColumn {
            name: col_name.to_string(),
            col_type: StoreColumnType::Text,
            default_value: "DEFAULT".to_string(),
        };
        self.columns.push(col);
        self
    }

    /// Build and initialize the store.
    pub async fn build(self) -> Result<ManagedStore> {
        // Must have at least one column => key
        // But we will add that ourselves if not present.
        // We forcibly add "key TEXT PRIMARY KEY".
        // Then for each user col, we create columns (like "mycol BIGINT DEFAULT 0" or "mycol TEXT DEFAULT ''").
        let store_name = self.store_name;
        if store_name.is_empty() {
            panic!("Must specify store name before building the ManagedStore.");
        }

        // We always create a "key TEXT PRIMARY KEY"
        // Then each user col is "col_name BIGINT DEFAULT 0" or "col_name TEXT DEFAULT ''", etc.
        let mut definition_lines = vec!["key TEXT PRIMARY KEY".to_string()];
        for c in &self.columns {
            let line = match c.col_type {
                StoreColumnType::Numeric => {
                    format!("{} BIGINT DEFAULT 0", c.name)
                }
                StoreColumnType::Text => {
                    format!("{} TEXT DEFAULT ''", c.name)
                }
            };
            definition_lines.push(line);
        }

        // If not preserve_data => drop the table
        if !self.preserve_data {
            let drop_q = format!("DROP TABLE IF EXISTS {} CASCADE;", store_name);
            self.client.execute(&drop_q, &[]).await?;
        }

        // create it
        let create_q = format!("CREATE TABLE IF NOT EXISTS {} ( {} );", store_name, definition_lines.join(", "));
        self.client.execute(&create_q, &[]).await?;

        Ok(ManagedStore {
            client: self.client,
            store_name,
            columns: self.columns,
        })
    }
}

#[cfg(test)]
mod managed_store_tests {
    use super::ManagedStore;
    use crate::constants::POSTGRES_CONNECTION_STRING; // Or wherever you keep this
    use anyhow::Result;
    use tokio_postgres::{
        types::{FromSql, ToSql},
        NoTls,
    };

    /// Connect to Postgres with the standard `POSTGRES_CONNECTION_STRING`.
    /// Spawns the connection handling on a background task.
    async fn connect_to_db() -> tokio_postgres::Client {
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to PostgreSQL");

        // Spawn the connection so errors are logged if they occur.
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        client
    }

    /// Utility for counting rows in the store by a given `key`.
    async fn row_count_for_key(client: &tokio_postgres::Client, table: &str, key: &str) -> i64 {
        let sql = format!("SELECT COUNT(*)::BIGINT FROM {} WHERE key = $1", table);
        let row = client.query_one(&sql, &[&key]).await.expect("Failed to query row_count_for_key");
        row.get::<_, i64>(0)
    }

    async fn get_column_value<T>(client: &tokio_postgres::Client, table: &str, key: &str, col: &str) -> Option<T>
    where
        // We need a higher-ranked trait bound here:
        T: for<'a> FromSql<'a> + std::marker::Unpin,
    {
        let sql = format!("SELECT {col} FROM {table} WHERE key = $1");
        let row_opt = client.query_opt(&sql, &[&key]).await.expect("Failed to query get_column_value");

        row_opt.map(|row| row.get::<_, T>(0))
    }

    /// ### Test 1: Basic creation + upsert flow
    ///
    /// This test verifies that we can:
    ///  1) Create a store with one numeric column and one text column
    ///  2) Upsert a new key with some partial data
    ///  3) Upsert the same key with updates to only some columns
    ///  4) Confirm results via direct SQL queries
    #[tokio::test]
    async fn test_managed_store_basic_upsert() -> Result<()> {
        // 1) Connect
        let client = connect_to_db().await;

        // 2) Build the store (non-preserving, so we drop if exists)
        let store_name = "test_managed_store_basic_upsert";
        let managed_store = ManagedStore::builder(client)
            .name(store_name)
            .add_numeric_column("counter")
            .add_text_column("comment")
            .build()
            .await
            .expect("Failed to build ManagedStore");

        // 3) Upsert #1 => Insert a brand-new key with partial columns
        // Let's specify only the "counter" column => the "comment" remains the default ('').
        managed_store
            .upsert(
                "my_key", // Row's key
                &[
                    ("counter", &123_i64 as &(dyn ToSql + Sync)), /* We set counter=123
                                                                   * no "comment" */
                ],
            )
            .await
            .expect("Failed upsert #1");

        // Verify we have exactly 1 row for "my_key"
        let count = row_count_for_key(managed_store.client(), store_name, "my_key").await;
        assert_eq!(count, 1, "Expected exactly 1 row for 'my_key' after upsert #1");

        // 3a) Check that "counter" is 123, "comment" is empty string
        let cval: i64 = get_column_value(managed_store.client(), store_name, "my_key", "counter")
            .await
            .expect("Missing column 'counter'");
        assert_eq!(cval, 123, "counter should be 123 after upsert #1");

        let comm_opt: Option<String> = get_column_value(managed_store.client(), store_name, "my_key", "comment").await;
        assert!(comm_opt.is_some());
        assert_eq!(comm_opt.unwrap(), "", "comment should be the default empty string after upsert #1");

        // 4) Upsert #2 => We update only "comment" => do not supply "counter"
        managed_store
            .upsert(
                "my_key",
                &[
                    ("comment", &"HelloWorld" as &(dyn ToSql + Sync)),
                    // no "counter"
                ],
            )
            .await
            .expect("Failed upsert #2");

        // 4a) Check that "comment" is updated, "counter" remains 123
        let cval2: i64 = get_column_value(managed_store.client(), store_name, "my_key", "counter")
            .await
            .expect("Missing column 'counter' after upsert #2");
        assert_eq!(cval2, 123, "counter should remain 123 after upsert #2");

        let comm2_opt: Option<String> = get_column_value(managed_store.client(), store_name, "my_key", "comment").await;
        assert_eq!(comm2_opt.unwrap(), "HelloWorld", "comment should be updated to 'HelloWorld' after upsert #2");

        Ok(())
    }

    /// ### Test 2: `preserve_data` set to true
    ///
    /// 1) Create a store (with preserve_data=false, the default), upsert a row
    /// 2) Rebuild the store with `preserve_data=true` => confirm the row is preserved
    /// 3) Insert a new row => confirm it coexists with the old row
    #[tokio::test]
    async fn test_managed_store_preserve_data() -> Result<()> {
        // 1) Connect
        let client = connect_to_db().await;

        // 2) Build the store with preserve_data=false (the default)
        let store_name = "test_managed_store_preserve_data";
        {
            let store = ManagedStore::builder(client)
                .name(store_name)
                .add_numeric_column("num_col")
                .add_text_column("txt_col")
                // .preserve_data() // not called => false
                .build()
                .await
                .expect("Failed to build store #1");

            // Upsert a row
            store
                .upsert("keyA", &[("num_col", &10_i64 as &(dyn ToSql + Sync))])
                .await
                .expect("Upsert #1 failed");
        }

        // 3) Re-connect to DB for the second pass
        let client2 = connect_to_db().await;
        {
            // Build again, but this time preserve_data()
            let store2 = ManagedStore::builder(client2)
                .name(store_name)
                .add_numeric_column("num_col")
                .add_text_column("txt_col")
                .preserve_data()
                .build()
                .await
                .expect("Failed to build store #2 with preserve_data");

            // Confirm that "keyA" row remains
            let old_count = row_count_for_key(store2.client(), store_name, "keyA").await;
            assert_eq!(old_count, 1, "Expected row for 'keyA' to remain due to preserve_data = true");

            // Insert a second row, "keyB"
            store2
                .upsert("keyB", &[("num_col", &42_i64 as &(dyn ToSql + Sync)), ("txt_col", &"some_text")])
                .await
                .expect("Upsert #2 for 'keyB' failed");

            // Confirm that we now have 2 rows total: keyA + keyB
            let count_a = row_count_for_key(store2.client(), store_name, "keyA").await;
            let count_b = row_count_for_key(store2.client(), store_name, "keyB").await;
            assert_eq!(count_a, 1, "Still should have keyA");
            assert_eq!(count_b, 1, "Should have inserted keyB as well");
        }

        Ok(())
    }

    /// ### Test 3: Upserting a key with no columns => defaults
    ///
    /// This ensures that if you do an upsert with no columns,
    /// it inserts a row with default values for all user-defined columns.
    #[tokio::test]
    async fn test_managed_store_upsert_no_columns() -> Result<()> {
        // 1) Connect
        let client = connect_to_db().await;

        // 2) Build store
        let store_name = "test_managed_store_upsert_no_cols";
        let store = ManagedStore::builder(client)
            .name(store_name)
            .add_numeric_column("mycount")
            .add_text_column("description")
            .build()
            .await
            .expect("Failed to build store");

        // 3) Upsert a key with zero columns
        store.upsert("keyX", &[]).await.expect("Upsert with empty columns fails?");

        // 4) Query => we have 1 row, mycount=0, desc=''
        let row_count = row_count_for_key(store.client(), store_name, "keyX").await;
        assert_eq!(row_count, 1, "Should have inserted one row for keyX with all defaults");

        let mycount: i64 = get_column_value(store.client(), store_name, "keyX", "mycount").await.unwrap();
        assert_eq!(mycount, 0, "mycount should default to 0");

        let desc: String = get_column_value(store.client(), store_name, "keyX", "description").await.unwrap();
        assert_eq!(desc, "", "desc should default to empty string");

        // 5) Upsert again => no columns => does nothing
        store.upsert("keyX", &[]).await.expect("Second upsert with no columns?");

        // Should remain exactly the same
        let row_count2 = row_count_for_key(store.client(), store_name, "keyX").await;
        assert_eq!(row_count2, 1, "Still only one row for keyX after second upsert");

        Ok(())
    }
}
