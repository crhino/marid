use std::thread;
use std::sync::mpsc;
use chan;
use std::error::Error;
use std::fmt;
use {Signal, Process, Runner, Receiver, Sender};

pub struct TestRunner {
    data: usize,
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
    pub fn new(data: usize) -> TestRunner {
        TestRunner { data: data }
    }
}

impl Runner for TestRunner {
    type Error = TestError;

    fn setup(&mut self) -> Result<(), TestError> {
        self.data = 27;
        Ok(())
    }

    fn run(mut self, signals: Receiver<Signal>) -> Result<(), TestError> {
        self.data += 73;
        let sig = signals.recv().unwrap();
        if sig == Signal::INT {
            Ok(())
        } else {
            Err(TestError)
        }
    }
}

pub struct TestProcess {
    runner: mpsc::Receiver<bool>,
    signals: Sender<Signal>,
}

impl TestProcess {
    pub fn new(runner: TestRunner) -> TestProcess {
        let (sn, rc) = mpsc::channel();
        let (send_runner, recv_runner) = mpsc::channel::<TestRunner>();
        // Marid sender/receiver
        let (signals, sig_recv) = chan::async();
        thread::spawn(move || {
            let mut runner = recv_runner.recv().unwrap();
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
