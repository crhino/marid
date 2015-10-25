#![deny(missing_docs, dead_code)]
//! Marid
//!
//! A process orchestration library
#[macro_use]
extern crate chan;
extern crate chan_signal;
extern crate crossbeam;

use std::error::Error;

mod traits;
pub use traits::*;

mod composer;
pub use composer::Composer;

mod process;
use process::{ProcessError, MaridProcess};

/// This function will start the specified runner as well as listen on the specified
/// signals.
///
/// # Warnings
///
/// This must be called before any threads are spawned in the process to
/// ensure appropriate signal handling behavior.
pub fn initiate<R, E>(runner: R, signals: Vec<Signal>) -> Box<Process<Error=ProcessError<E>>>
where R: Runner<Error=E> + Send + 'static, E: Error + Send {
    let (signal_send, signal_recv) = chan::sync(1024);
    for sig in signals {
        chan_signal::notify_on(&signal_send, sig);
    }

    Box::new(MaridProcess::new(runner, signal_send, signal_recv))
}

// TODO: Make this module more useable and document behavior.
pub mod test_helpers;
