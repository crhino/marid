pub use chan_signal::Signal;
pub use chan::{Sender, Receiver};
use {MaridError};

/// A type implementing the Runner trait has the job of performing some arbitrary
/// work while waiting for a signal indication shutdown. Upon receiving that
/// defined shutdown Signal, the Runner must exit in a finite period of time.
pub trait Runner {
    /// Performs work for an indefinite amount of time.
    ///
    /// The Box<Self> form is used here in order to allow Process types the ability to run
    /// different types of Runners at once.
    fn run(self: Box<Self>, signals: Receiver<Signal>) -> Result<(), MaridError>;

    /// Used to do any setup work necessary for the Runner.
    ///
    /// This function should only complete once the type is ready to be run,
    /// and must complete in a finite period of time.
    fn setup(&mut self) -> Result<(), MaridError>;
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
    /// ### Warnings
    /// This must be a non-blocking function.
    fn signal(&self, signal: Signal);
}

#[cfg(test)]
mod tests {
    use {Signal, Process};
    use test_helpers::{TestProcess, TestRunner};
    use chan;

    #[test]
    fn test_runner_and_thread() {
        let (sn, rc) = chan::sync(1);
        let runner = TestRunner::new(0, sn);

        let thread = TestProcess::new(runner);
        assert!(thread.ready().is_ok());
        thread.signal(Signal::INT);
        assert!(thread.wait().is_ok());

        assert!(rc.recv().unwrap());
    }

    #[test]
    fn test_signal() {
        let (sn, rc) = chan::sync(1);
        let runner = TestRunner::new(0, sn);

        let thread = TestProcess::new(runner);
        assert!(thread.ready().is_ok());
        thread.signal(Signal::HUP);
        assert!(thread.wait().is_err());

        assert!(!rc.recv().unwrap());
    }
}
