use thunk::Thunk;
use traits::{Receiver};
use {MaridError, Runner, Signal};

/// A Runner type that is constructed with a FnOnce closure.
pub type FnRunner = Thunk<'static, Receiver<Signal>, Result<(), MaridError>>;

impl FnRunner {
    /// Create a new FnRunner
    pub fn new<F>(func: F) -> FnRunner
        where F: FnOnce(Receiver<Signal>) -> Result<(), MaridError>, F: Send + 'static {
            Thunk::with_arg(func)
        }
}

impl Runner for FnRunner {
    fn run(self: Box<Self>, signals: Receiver<Signal>) -> Result<(), MaridError> {
        (*self).invoke(signals)
    }

    fn setup(&mut self) -> Result<(), MaridError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {Runner, FnRunner};
    use chan;

    #[test]
    fn test_thunk_runner() {
        let (_sn, rc) = chan::sync(1);
        let mut runner = Box::new(FnRunner::new(move |_sig| {
            Ok(())
        }));

        assert!(runner.setup().is_ok());
        assert!(runner.run(rc).is_ok());
    }
}
