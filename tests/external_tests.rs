#![cfg(test)]

extern crate marid;
extern crate chan;

use marid::test_helpers::{TestRunner};
use marid::{initiate, Runner, MaridError, Composer, Process, Receiver, Signal, ProcessError};

#[derive(Debug, Eq, PartialEq, Clone)]
struct NullRunner;
impl Runner for NullRunner {
    fn setup(&mut self) -> Result<(), MaridError> { Ok(()) }

    fn run(self: Box<Self>, _signals: Receiver<Signal>) -> Result<(), MaridError> {
        Ok(())
    }
}


#[test]
fn test_initiate() {
    let (sn1, rc1) = chan::sync(0);
    let runner1 = Box::new(TestRunner::new(1, sn1.clone())) as Box<Runner + Send>;
    let (sn2, rc2) = chan::sync(0);
    let runner2 = Box::new(TestRunner::new(2, sn2.clone())) as Box<Runner + Send>;

    let composer = Composer::new(vec!(runner1, runner2));
    let signals = vec!(Signal::INT, Signal::ALRM);

    let process = initiate(composer, signals);

    assert!(process.ready().is_ok());

    process.signal(Signal::INT);
    assert!(rc1.recv().unwrap()); // Rendevous channels
    assert!(rc2.recv().unwrap());

    assert!(process.wait().is_ok());
}

#[test]
fn test_initiate_error() {
    let (sn1, rc1) = chan::sync(0);
    let runner1 = Box::new(TestRunner::new(1, sn1.clone())) as Box<Runner + Send>;
    let (sn2, rc2) = chan::sync(0);
    let runner2 = Box::new(TestRunner::new(2, sn2.clone())) as Box<Runner + Send>;

    let composer = Composer::new(vec!(runner1, runner2));
    let signals = vec!(Signal::INT, Signal::HUP);

    let process = initiate(composer, signals);

    assert!(process.ready().is_ok());

    process.signal(Signal::HUP);
    assert!(!rc1.recv().unwrap()); // Rendevous channels
    assert!(!rc2.recv().unwrap());

    match process.wait() {
        Ok(_) => assert!(false, "Expected an error"),
        Err(ProcessError::RunnerError(_)) => {},
        Err(_) => assert!(false, "Wrong error type"),
    }
}

#[test]
fn test_initiate_different_runners() {
    let (sn1, rc1) = chan::sync(0);
    let runner1 = Box::new(TestRunner::new(1, sn1.clone())) as Box<Runner + Send>;
    let runner2 = Box::new(NullRunner) as Box<Runner + Send>;

    let composer = Composer::new(vec!(runner1, runner2));
    let signals = vec!(Signal::INT, Signal::HUP);

    let process = initiate(composer, signals);

    assert!(process.ready().is_ok());

    process.signal(Signal::HUP);
    assert!(!rc1.recv().unwrap()); // Rendevous channels

    match process.wait() {
        Ok(_) => assert!(false, "Expected an error"),
        Err(ProcessError::RunnerError(_)) => {},
        Err(_) => assert!(false, "Wrong error type"),
    }
}

