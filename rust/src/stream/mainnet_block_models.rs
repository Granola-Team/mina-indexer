use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct MainnetBlock {
    pub scheduled_time: String,
    pub protocol_state: ProtocolState,
    pub staged_ledger_diff: Option<StagedLedgerDiff>,
}

impl MainnetBlock {
    pub fn get_previous_state_hash(&self) -> String {
        self.protocol_state.previous_state_hash.clone()
    }

    pub fn get_last_vrf_output(&self) -> String {
        self.protocol_state.body.consensus_state.last_vrf_output.clone()
    }

    // Calculates the count of commands in `diff[0].commands` and `diff[1].commands`
    pub fn get_user_commands_count(&self) -> usize {
        let diff_0_count = self
            .staged_ledger_diff
            .as_ref()
            .and_then(|ledger_diff| ledger_diff.diff.get(0)) // Access `diff[0]`
            .and_then(|opt_diff| opt_diff.as_ref()) // Check if `diff[0]` is `Some`
            .map(|diff| diff.commands.len()) // Get the length of commands array
            .unwrap_or(0); // Default to 0 if `diff[0]` is None

        let diff_1_count = self
            .staged_ledger_diff
            .as_ref()
            .and_then(|ledger_diff| ledger_diff.diff.get(1)) // Access `diff[1]`
            .and_then(|opt_diff| opt_diff.as_ref()) // Check if `diff[1]` is `Some`
            .map(|diff| diff.commands.len()) // Get the length of commands array
            .unwrap_or(0); // Default to 0 if `diff[1]` is None

        diff_0_count + diff_1_count
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolState {
    pub previous_state_hash: String,
    pub body: Body,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Body {
    pub consensus_state: ConsensusState,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ConsensusState {
    pub last_vrf_output: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StagedLedgerDiff {
    pub diff: Vec<Option<Diff>>, // Vector of Option<Diff> to handle potential null entries
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Diff {
    pub commands: Vec<Command>, // Each `Diff` contains a vector of `Command`s
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Command {
    pub data: Vec<sonic_rs::Value>, // Placeholder type to avoid parsing nested structures now
}
