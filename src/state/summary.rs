use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use std::{str::Lines, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub block_count: usize,
    pub date_time: DateTime<Utc>,
    pub uptime: Duration,
    pub root_height: usize,
    pub root_length: usize,
    pub num_dangling: usize,
    pub max_dangling_height: usize,
    pub max_dangling_length: usize,
    pub db_stats: Option<DbStats>,
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "===== Mina-indexer summary =====")?;
        writeln!(
            f,
            r#"
Blocks included: {}
Start date:      {}
Uptime:          {:?}
"#,
            self.block_count, self.date_time, self.uptime,
        )?;

        write!(f, "=== Root branch ===")?;
        writeln!(
            f,
            r#"
Height: {}
Length: {}
"#,
            self.root_height, self.root_length,
        )?;

        write!(f, "=== Dangling branches ===")?;
        writeln!(
            f,
            r#"
Num:        {}
Max height: {}
Max length: {}
"#,
            self.num_dangling, self.max_dangling_height, self.max_dangling_length,
        )?;

        if let Some(db_stats) = self.db_stats.as_ref() {
            write!(f, "=== DB stats ===")?;
            writeln!(
                f,
                r#"
Uptime:            {}
Cumulative writes: {}
Cumulative WAL:    {}
Cumulative stall:  {}
Interval writes:   {}
Interval WAL:      {}
Interval stall:    {}
"#,
                db_stats.uptime,
                db_stats.cum_writes,
                db_stats.cum_wal,
                db_stats.cum_stall,
                db_stats.int_writes,
                db_stats.int_wal,
                db_stats.int_stall,
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    uptime: String,
    cum_writes: String,
    cum_wal: String,
    cum_stall: String,
    int_writes: String,
    int_wal: String,
    int_stall: String,
}

impl std::str::FromStr for DbStats {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lines = s.lines();
        lines.next().unwrap(); // skip header
        Ok(DbStats {
            uptime: value(&mut lines),
            cum_writes: value(&mut lines),
            cum_wal: value(&mut lines),
            cum_stall: value(&mut lines),
            int_writes: value(&mut lines),
            int_wal: value(&mut lines),
            int_stall: value(&mut lines),
        })
    }
}

fn value(lines: &mut Lines) -> String {
    let mut res = String::new();
    let mut split_line = lines.next().unwrap().split(':').collect::<Vec<&str>>();
    split_line.remove(0);
    for s in split_line {
        res.push_str(s.trim_start());
    }
    res
}
