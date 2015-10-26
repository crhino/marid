//! A testing module for Runner/Process traits.
//!
//! When implementing different pieces of this crate, it was expedient
//! to have some testing structs in place. This might also be useful for
//! other developers who would want to ensure proper functionality.
use std::thread;
use std::sync::mpsc;
use chan;
use std::error::Error;
use std::fmt;
use {MaridError, Signal, Process, Runner, Receiver, Sender};

/// A test struct that implements the Runner trait.
pub struct TestRunner {
    data: usize,
    sender: Sender<bool>
}

#[derive(Debug, Eq, PartialEq, Clone)]
struct TestError;

impl fmt::Display for TestError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "a testing error")
    }
}
impl Error for TestError {
    fn description(&self) -> &str {
        "a testing error"
    }
}

impl TestRunner {
    /// Create a new TestRunner.
    pub fn new(data: usize, sender: Sender<bool>) -> TestRunner {
        TestRunner {
            data: data,
            sender: sender,
        }
    }
}

impl Runner for TestRunner {
    fn setup(&mut self) -> Result<(), MaridError> {
        self.data = 27;
        Ok(())
    }

    fn run(mut self: Box<Self>, signals: Receiver<Signal>) -> Result<(), MaridError> {
        self.data += 73;
        assert_eq!(self.data, 100);
        let sig = signals.recv().expect("Could not recv signal");
        if sig == Signal::INT {
            self.sender.send(true);
            Ok(())
        } else {
            self.sender.send(false);
            Err(Box::new(TestError))
        }
    }
}

/// A test struct that implements the Process trait.
pub struct TestProcess {
    runner: mpsc::Receiver<bool>,
    signals: Sender<Signal>,
}

impl TestProcess {
    /// Create a new TestProcess.
    pub fn new(runner: TestRunner) -> TestProcess {
        let (sn, rc) = mpsc::channel();
        let (send_runner, recv_runner) = mpsc::channel::<TestRunner>();
        // Marid sender/receiver
        let (signals, sig_recv) = chan::async();
        thread::spawn(move || {
            let mut runner = Box::new(recv_runner.recv().unwrap());
            match runner.setup() {
                Ok(_) => sn.send(true),
                Err(_) => sn.send(false),
            }.unwrap();

            match runner.run(sig_recv) {
                Ok(_) => sn.send(true),
                Err(_) => sn.send(false),
            }.unwrap();
        });

        send_runner.send(runner).unwrap();

        TestProcess{
            runner: rc,
            signals: signals,
        }
    }
}

impl Process for TestProcess {
    type Error = TestError;

    fn ready(&self) -> Result<(), Self::Error> {
        if self.runner.recv().unwrap() {
            Ok(())
        } else {
            Err(TestError)
        }
    }

    fn wait(&self) -> Result<(), Self::Error> {
        match self.runner.recv() {
            Ok(b) => if b { Ok(()) } else { Err(TestError) },
            Err(_) => Err(TestError),
        }
    }

    fn signal(&self, signal: Signal) {
        self.signals.send(signal);
    }
}
