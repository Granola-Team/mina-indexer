use bytesize::ByteSize;
use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use std::{str::Lines, time::Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub uptime: Duration,
    pub date_time: DateTime<Utc>,
    pub blocks_processed: u32,
    pub best_tip_hash: String,
    pub root_hash: String,
    pub root_height: usize,
    pub root_length: usize,
    pub num_leaves: usize,
    pub num_dangling: usize,
    pub max_dangling_height: usize,
    pub max_dangling_length: usize,
    pub db_stats: Option<DbStats>,
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "===== Mina-indexer summary =====")?;
        writeln!(f, "  Uptime:       {:?}", self.uptime)?;
        writeln!(f, "  Started:      {}", self.date_time)?;
        writeln!(f, "  Blocks added: {}", self.blocks_processed)?;

        writeln!(f, "\n=== Root branch ===")?;
        writeln!(f, "  Height:        {}", self.root_height)?;
        writeln!(f, "  Length:        {}", self.root_length)?;
        writeln!(f, "  Num leaves:    {}", self.num_leaves)?;
        writeln!(f, "  Root hash:     {}", self.root_hash)?;
        writeln!(f, "  Best tip hash: {}", self.best_tip_hash)?;

        if self.num_dangling > 0 {
            writeln!(f, "\n=== Dangling branches ===")?;
            writeln!(f, "  Num:        {}", self.num_dangling)?;
            writeln!(f, "  Max height: {}", self.max_dangling_length)?;
            writeln!(f, "  Max length: {}", self.max_dangling_height)?;
        }

        let db_stats = self.db_stats.as_ref().unwrap();
        writeln!(f, "\n=== DB stats ===")?;
        writeln!(f, "  All memtable size: {}", ByteSize::b(db_stats.memory))?;
        writeln!(f, "  Uptime:            {}", db_stats.uptime)?;
        writeln!(f, "  Cumulative writes: {}", db_stats.cum_writes)?;
        writeln!(f, "  Cumulative WAL:    {}", db_stats.cum_wal)?;
        writeln!(f, "  Cumulative stall:  {}", db_stats.cum_stall)?;
        writeln!(f, "  Interval writes:   {}", db_stats.int_writes)?;
        writeln!(f, "  Interval WAL:      {}", db_stats.int_wal)?;
        writeln!(f, "  Interval stall:    {}", db_stats.int_stall)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbStats {
    memory: u64,
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
        let memory = lines.next().unwrap().parse::<u64>()?;
        lines.nth(1).unwrap(); // skip header
        Ok(DbStats {
            memory,
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
    let line = lines.next().unwrap();
    let idx = line.find(':').unwrap();
    res.push_str(line[(idx + 1)..].trim_start());
    res
}
