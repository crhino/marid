#![deny(missing_docs, dead_code)]
//! Marid
//!
//! A process orchestration library

#[macro_use]
extern crate chan;
extern crate chan_signal;
mod traits;
pub use traits::*;

#[cfg(test)]
mod test_helpers;

// mod composer;
