// #![deny(missing_docs, dead_code)]
//! Marid
//!
//! A process orchestration library
//!
//! Currently this is only available on the nightly branch, waiting for ferrous_thread
//! ThreadPool to be able to be used on stable branch.

#[macro_use]
extern crate chan;
extern crate chan_signal;
extern crate crossbeam;

mod traits;
pub use traits::*;

mod composer;
pub use composer::Composer;

#[cfg(test)]
mod test_helpers;
