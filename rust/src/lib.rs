extern crate core;

pub mod block;
pub mod canonicity;
pub mod chain;
pub mod client;
mod collection;
pub mod command;
pub mod constants;
pub mod event;
pub mod ledger;
pub mod mina_blocks;
pub mod proof_systems;
pub mod protocol;
pub mod server;
pub mod snark_work;
pub mod state;
pub mod store;
pub mod unix_socket_server;
pub mod web;

#[cfg(target_family = "unix")]
pub mod platform {
    use libc::{kill, pid_t};

    pub fn is_process_running(pid: pid_t) -> bool {
        // kill(pid, 0) sends signal 0 to the process, which is a no-op check
        // If the process exists, kill() returns 0, otherwise it returns -1
        unsafe { kill(pid, 0) == 0 }
    }
}
