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
    /// and must complete in a finite period of time.
    fn setup(&mut self) -> Result<(), Self::Error>;
}

/// A Process represents are running unit of work. It can be signaled and waited on.
pub trait Process {
    /// Error type for the Process.
    type Error;

    /// This function will block until the running Process has finished its setup and
    /// is ready to run.
    fn ready(&self) -> Result<(), Self::Error>;
    /// This function will wait until the Process has exited, returning a success or
    /// failure.
    fn wait(&self) -> Result<(), Self::Error>;
    /// This function will signal the running Process with the specified signal.
    ///
    /// This is a non-blocking function.
    fn signal(&self, signal: Signal);
}

#[cfg(test)]
mod tests {
    use {Signal, Process};
    use test_helpers::{TestProcess, TestRunner};

    #[test]
    fn test_runner_and_thread() {
        let runner = TestRunner::new(0);
        let thread = TestProcess::new(runner);
        assert!(thread.ready().is_ok());
        thread.signal(Signal::INT);
        assert!(thread.wait().is_ok());
    }

    #[test]
    fn test_signal() {
        let runner = TestRunner::new(0);
        let thread = TestProcess::new(runner);
        assert!(thread.ready().is_ok());
        thread.signal(Signal::HUP);
        assert!(thread.wait().is_err());
    }
}
