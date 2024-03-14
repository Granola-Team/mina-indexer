use bytesize::ByteSize;
use serde::{Deserialize, Serialize};
use std::str::Lines;

pub trait Summary {
    fn uptime(&self) -> std::time::Duration;
    fn blocks_processed(&self) -> u32;
    fn bytes_processed(&self) -> bytesize::ByteSize;
    fn best_tip_length(&self) -> u32;
    fn best_tip_hash(&self) -> String;
    fn canonical_root_length(&self) -> u32;
    fn canonical_root_hash(&self) -> String;
    fn root_hash(&self) -> String;
    fn root_height(&self) -> u32;
    fn root_length(&self) -> u32;
    fn num_leaves(&self) -> u32;
    fn num_dangling(&self) -> u32;
    fn max_dangling_height(&self) -> u32;
    fn max_dangling_length(&self) -> u32;
    fn db_stats(&self) -> DbStats;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryShort {
    pub uptime: std::time::Duration,
    pub blocks_processed: u32,
    pub bytes_processed: u64,
    pub witness_tree: WitnessTreeSummaryShort,
    pub db_stats: Option<DbStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryVerbose {
    pub uptime: std::time::Duration,
    pub blocks_processed: u32,
    pub bytes_processed: u64,
    pub witness_tree: WitnessTreeSummaryVerbose,
    pub db_stats: Option<DbStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessTreeSummaryShort {
    pub best_tip_length: u32,
    pub best_tip_hash: String,
    pub canonical_root_length: u32,
    pub canonical_root_hash: String,
    pub root_hash: String,
    pub root_height: u32,
    pub root_length: u32,
    pub num_leaves: u32,
    pub num_dangling: u32,
    pub max_dangling_height: u32,
    pub max_dangling_length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessTreeSummaryVerbose {
    pub best_tip_length: u32,
    pub best_tip_hash: String,
    pub canonical_root_length: u32,
    pub canonical_root_hash: String,
    pub root_hash: String,
    pub root_height: u32,
    pub root_length: u32,
    pub num_leaves: u32,
    pub num_dangling: u32,
    pub max_dangling_height: u32,
    pub max_dangling_length: u32,
    pub witness_tree: String,
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

impl std::fmt::Display for SummaryShort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        summary_short(self, f)
    }
}

impl std::fmt::Display for SummaryVerbose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        summary_short(self, f)?;
        writeln!(f, "\n===== Witness tree =====")?;
        write!(f, "{}", self.witness_tree.witness_tree)?;
        Ok(())
    }
}

impl From<SummaryVerbose> for SummaryShort {
    fn from(value: SummaryVerbose) -> Self {
        Self {
            uptime: value.uptime,
            blocks_processed: value.blocks_processed,
            bytes_processed: value.bytes_processed,
            witness_tree: value.witness_tree.into(),
            db_stats: value.db_stats,
        }
    }
}

impl From<WitnessTreeSummaryVerbose> for WitnessTreeSummaryShort {
    fn from(value: WitnessTreeSummaryVerbose) -> Self {
        Self {
            best_tip_length: value.best_tip_length,
            best_tip_hash: value.best_tip_hash,
            canonical_root_length: value.canonical_root_length,
            canonical_root_hash: value.canonical_root_hash,
            root_hash: value.root_hash,
            root_height: value.root_height,
            root_length: value.root_length,
            num_leaves: value.num_leaves,
            num_dangling: value.num_dangling,
            max_dangling_height: value.max_dangling_height,
            max_dangling_length: value.max_dangling_length,
        }
    }
}

fn summary_short(state: &impl Summary, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "===== Mina-indexer summary =====")?;
    writeln!(f, "  Uptime:       {:?}", state.uptime())?;
    writeln!(f, "  Bytes added:  {}", state.bytes_processed())?;
    writeln!(f, "  Blocks added: {}", state.blocks_processed())?;

    writeln!(f, "\n=== Root branch ===")?;
    writeln!(f, "  Height:                {}", state.root_height())?;
    writeln!(f, "  Length:                {}", state.root_length())?;
    writeln!(f, "  Num leaves:            {}", state.num_leaves())?;
    writeln!(f, "  Root hash:             {}", state.root_hash())?;
    writeln!(f, "  Best tip length:       {}", state.best_tip_length())?;
    writeln!(f, "  Best tip hash:         {}", state.best_tip_hash())?;
    writeln!(
        f,
        "  Canonical root length: {}",
        state.canonical_root_length()
    )?;
    writeln!(
        f,
        "  Canonical root hash:   {}",
        state.canonical_root_hash()
    )?;

    if state.num_dangling() > 0 {
        writeln!(f, "\n=== Dangling branches ===")?;
        writeln!(f, "  Num:        {}", state.num_dangling())?;
        writeln!(f, "  Max height: {}", state.max_dangling_length())?;
        writeln!(f, "  Max length: {}", state.max_dangling_height())?;
    }

    // let db_stats = state.db_stats.as_ref().unwrap();
    writeln!(f, "\n=== DB stats ===")?;
    writeln!(
        f,
        "  All memtable size: {}",
        ByteSize::b(state.db_stats().memory)
    )?;
    writeln!(f, "  Uptime:            {}", state.db_stats().uptime)?;
    writeln!(f, "  Cumulative writes: {}", state.db_stats().cum_writes)?;
    writeln!(f, "  Cumulative WAL:    {}", state.db_stats().cum_wal)?;
    writeln!(f, "  Cumulative stall:  {}", state.db_stats().cum_stall)?;
    writeln!(f, "  Interval writes:   {}", state.db_stats().int_writes)?;
    writeln!(f, "  Interval WAL:      {}", state.db_stats().int_wal)?;
    writeln!(f, "  Interval stall:    {}", state.db_stats().int_stall)?;

    Ok(())
}

impl Summary for SummaryShort {
    fn best_tip_hash(&self) -> String {
        self.witness_tree.best_tip_hash.clone()
    }

    fn best_tip_length(&self) -> u32 {
        self.witness_tree.best_tip_length
    }

    fn blocks_processed(&self) -> u32 {
        self.blocks_processed
    }

    fn bytes_processed(&self) -> bytesize::ByteSize {
        bytesize::ByteSize(self.bytes_processed)
    }

    fn canonical_root_hash(&self) -> String {
        self.witness_tree.canonical_root_hash.clone()
    }

    fn canonical_root_length(&self) -> u32 {
        self.witness_tree.canonical_root_length
    }

    fn db_stats(&self) -> DbStats {
        self.db_stats.as_ref().unwrap().clone()
    }

    fn max_dangling_height(&self) -> u32 {
        self.witness_tree.max_dangling_height
    }

    fn max_dangling_length(&self) -> u32 {
        self.witness_tree.max_dangling_length
    }

    fn num_dangling(&self) -> u32 {
        self.witness_tree.num_dangling
    }

    fn num_leaves(&self) -> u32 {
        self.witness_tree.num_leaves
    }

    fn root_hash(&self) -> String {
        self.witness_tree.root_hash.clone()
    }

    fn root_height(&self) -> u32 {
        self.witness_tree.root_height
    }

    fn root_length(&self) -> u32 {
        self.witness_tree.root_length
    }

    fn uptime(&self) -> std::time::Duration {
        self.uptime
    }
}

impl Summary for SummaryVerbose {
    fn best_tip_hash(&self) -> String {
        self.witness_tree.best_tip_hash.clone()
    }

    fn best_tip_length(&self) -> u32 {
        self.witness_tree.best_tip_length
    }

    fn blocks_processed(&self) -> u32 {
        self.blocks_processed
    }

    fn bytes_processed(&self) -> bytesize::ByteSize {
        bytesize::ByteSize(self.bytes_processed)
    }

    fn canonical_root_hash(&self) -> String {
        self.witness_tree.canonical_root_hash.clone()
    }

    fn canonical_root_length(&self) -> u32 {
        self.witness_tree.canonical_root_length
    }

    fn db_stats(&self) -> DbStats {
        self.db_stats.as_ref().unwrap().clone()
    }

    fn max_dangling_height(&self) -> u32 {
        self.witness_tree.max_dangling_height
    }

    fn max_dangling_length(&self) -> u32 {
        self.witness_tree.max_dangling_length
    }

    fn num_dangling(&self) -> u32 {
        self.witness_tree.num_dangling
    }

    fn num_leaves(&self) -> u32 {
        self.witness_tree.num_leaves
    }

    fn root_hash(&self) -> String {
        self.witness_tree.root_hash.clone()
    }

    fn root_height(&self) -> u32 {
        self.witness_tree.root_height
    }

    fn root_length(&self) -> u32 {
        self.witness_tree.root_length
    }

    fn uptime(&self) -> std::time::Duration {
        self.uptime
    }
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
