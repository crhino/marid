pub use chan_signal::Signal;
pub use chan::{Sender, Receiver};

/// A type implementing the Runner trait has the job of performing some arbitrary
/// work while waiting for a signal indication shutdown. Upon receiving that
/// defined shutdown Signal, the Runner must exit in a finite period of time.
pub trait Runner {
    /// Error type for the Runner.
    type Error;

    /// The run function is called when a user wants to perform work.
    fn run(self, signals: Receiver<Signal>) -> Result<(), Self::Error>;

    /// The setup function is called when a user wants to get ready to work.
    ///
    /// This function should only complete once the type is ready to be run,
    /// and must complete in finite period of time.
    fn setup(&mut self) -> Result<(), Self::Error>;
}

/// A Thread represents are running unit of work. It can be signaled and waited on.
pub trait Thread {
    /// Error type for the Thread.
    type Error;

    /// This function will block until the running Thread has finished its setup and
    /// is ready to run.
    fn ready(&self) -> Result<(), Self::Error>;
    /// This function will wait until the Thread has exited, returning a success or
    /// failure.
    fn wait(&self) -> Result<(), Self::Error>;
    /// This function will signal the running Thread with the specified signal.
    ///
    /// This is a non-blocking function.
    fn signal(&self, signal: Signal);
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::sync::mpsc;
    use chan;
    use std::error::Error;
    use std::fmt;
    use {Signal, Thread, Runner, Receiver, Sender};

    struct Test {
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

    impl Runner for Test {
        type Error = TestError;

        fn setup(&mut self) -> Result<(), TestError> {
            self.data = 27;
            Ok(())
        }

        fn run(mut self, _signals: Receiver<Signal>) -> Result<(), TestError> {
            self.data += 73;
            Ok(())
        }
    }

    struct TestThread {
        runner: mpsc::Receiver<bool>,
        signals: Sender<Signal>,
    }

    impl TestThread {
        fn new(runner: Test) -> TestThread {
            let (sn, rc) = mpsc::channel();
            let (send_runner, recv_runner) = mpsc::channel::<Test>();
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

            TestThread{
                runner: rc,
                signals: signals,
            }
        }
    }

    impl Thread for TestThread {
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
                Ok(_) => Ok(()),
                Err(_) => Err(TestError),
            }
        }

        fn signal(&self, signal: Signal) {
            self.signals.send(signal);
        }
    }

    #[test]
    fn test_runner_and_thread() {
        let runner = Test { data: 0 };
        let thread = TestThread::new(runner);
        assert!(thread.ready().is_ok());
        assert!(thread.wait().is_ok());
    }

    #[test]
    fn test_signal() {
    }
}
