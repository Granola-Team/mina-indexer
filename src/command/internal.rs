use crate::{command::CommandUpdate, ledger::post_balances::PostBalance};

pub enum InternalCommand {
    User(CommandUpdate),
    Coinbase(PostBalance),
    FeeTransfer(FeeTransferUpdate),
}

pub enum FeeTransferUpdate {
    One(PostBalance),
    Two(PostBalance, PostBalance),
}
