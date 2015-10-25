#![cfg(test)]

extern crate marid;
extern crate chan;

use marid::test_helpers::{TestRunner};
use marid::{initiate, Composer, Process, Signal};

#[test]
fn test_initiate() {
    let (sn1, rc1) = chan::sync(0);
    let runner1 = TestRunner::new(0, sn1);
    let (sn2, rc2) = chan::sync(0);
    let runner2 = TestRunner::new(0, sn2);

    let composer = Composer::new(vec!(runner1, runner2));
    let signals = vec!(Signal::INT, Signal::ALRM);

    let process = initiate(composer, signals);

    assert!(process.ready().is_ok());

    process.signal(Signal::INT);
    assert!(rc1.recv().unwrap()); // Rendevous channels
    assert!(rc2.recv().unwrap());

    assert!(process.wait().is_ok());
}
