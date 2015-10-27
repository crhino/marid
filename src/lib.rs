#![deny(missing_docs, dead_code)]
//! Marid
//!
//! A process orchestration library
#[macro_use]
extern crate chan;
extern crate chan_signal;
extern crate crossbeam;

mod traits;
pub use traits::{Signal, Sender, Receiver, Process, Runner};

mod composer;
pub use composer::Composer;

mod process;
pub use process::{MaridProcess, ProcessError};

use std::error::Error;
/// Error type for Marid Runners.
pub type MaridError = Box<Error + Send>;

/// This function will start the specified runner as well as listen on the specified
/// signals.
///
/// # Warnings
///
/// This must be called before any threads are spawned in the process to
/// ensure appropriate signal handling behavior.
pub fn launch<R>(runner: R, signals: Vec<Signal>) -> MaridProcess
where R: Runner + Send + 'static {
    let (signal_send, signal_recv) = chan::sync(1024);
    for sig in signals {
        chan_signal::notify_on(&signal_send, sig);
    }

    MaridProcess::new(Box::new(runner), signal_send, signal_recv)
}

// TODO: Make this module more useable and document behavior.
pub mod test_helpers;
