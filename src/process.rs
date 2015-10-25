use std::thread;
use std::fmt;
use std::error::Error;
use std::sync::mpsc;
use std::cell::Cell;
use traits::{Runner, Process, Sender, Receiver, Signal};

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum ProcState {
    Init,
    SetupDone,
    Finished,
}


#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ProcessError<E> {
    RunnerError(E),
    ResultAlreadyGiven,
    CouldNotRecvResult,
}

impl<E: Error> fmt::Display for ProcessError<E> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ProcessError::RunnerError(ref e) => {
                write!(fmt, "{}", e.description())
            },
            ProcessError::ResultAlreadyGiven => {
                write!(fmt, "Already returned result to caller")
            },
            ProcessError::CouldNotRecvResult => {
                write!(fmt, "Could not receive result from thread")
            }
        }
    }
}

impl<E: Error> Error for ProcessError<E> {
    fn description(&self) -> &str {
        match *self {
            ProcessError::RunnerError(ref e) => {
                e.description()
            },
            ProcessError::ResultAlreadyGiven => {
                "Already returned result to caller"
            },
            ProcessError::CouldNotRecvResult => {
                "Could not receive result from thread"
            }
        }
    }
}

impl<E: Error> From<E> for ProcessError<E> {
    fn from(err: E) -> ProcessError<E> {
        ProcessError::RunnerError(err)
    }
}

pub struct MaridProcess<R: Runner> {
    setup_chan: mpsc::Receiver<Result<(), ProcessError<R::Error>>>,
    run_chan: mpsc::Receiver<Result<(), ProcessError<R::Error>>>,

    signaler: Sender<Signal>,
    runner: Option<thread::JoinHandle<()>>,
    state: Cell<ProcState>,
}

// Be aware, ready/wait
impl<R: Runner + Send + 'static> MaridProcess<R> {
    pub fn new(runner: R, signaler: Sender<Signal>, recv: Receiver<Signal>) -> MaridProcess<R> {
        let (setup_sn, setup_rc) = mpsc::channel();
        let (run_sn, run_rc) = mpsc::channel();

        let handle = spawn_run_thread(runner, recv, setup_sn, run_sn);

        MaridProcess {
            setup_chan: setup_rc,
            run_chan: run_rc,

            runner: Some(handle),
            signaler: signaler,
            state: Cell::new(ProcState::Init),
        }
    }
}

fn spawn_run_thread<R>(mut runner: R,
                       recv: Receiver<Signal>,
                       setup: mpsc::Sender<Result<(), ProcessError<R::Error>>>,
                       run: mpsc::Sender<Result<(), ProcessError<R::Error>>>) -> thread::JoinHandle<()>
where R: Runner + Send + 'static {
    thread::spawn(move || {
        let res = runner.setup().map_err(ProcessError::RunnerError);
        let is_err = res.is_err();
        setup.send(res).expect("Could not send setup result");

        if !is_err {
            let err = runner.run(recv).map_err(ProcessError::RunnerError);
            run.send(err).expect("Could not send run result");
        }
    })
}

impl<R: Runner> Process for MaridProcess<R> {
    type Error = ProcessError<R::Error>;

    fn ready(&self) -> Result<(), Self::Error> {
        match self.state.get() {
            ProcState::Init => {
                self.state.set(ProcState::SetupDone);
                self.setup_chan.recv().unwrap_or(Err(ProcessError::CouldNotRecvResult))
            },
            _ => {
                Err(ProcessError::ResultAlreadyGiven)
            }
        }
    }

    fn wait(&self) -> Result<(), Self::Error> {
        match self.state.get() {
            ProcState::Init | ProcState::SetupDone => {
                self.state.set(ProcState::Finished);
                self.run_chan.recv().unwrap_or(Err(ProcessError::CouldNotRecvResult))
            },
            ProcState::Finished => {
                Err(ProcessError::ResultAlreadyGiven)
            }
        }
    }

    fn signal(&self, signal: Signal) {
        self.signaler.send(signal)
    }
}

impl<R: Runner> Drop for MaridProcess<R> {
    fn drop(&mut self) {
        let runner = self.runner.take();
        runner.expect("No runner").
            join().expect("Runner panicked");
    }
}

#[cfg(test)]
mod tests {
    use test_helpers::{TestRunner};
    use super::{MaridProcess, ProcessError};
    use traits::{Process, Signal};
    use chan;

    #[test]
    fn test_ready_process() {
        let (sn, rc) = chan::sync(0);
        let runner = TestRunner::new(0, sn);

        let (signal_sn, signal_rc) = chan::sync(9);
        let process = MaridProcess::new(runner, signal_sn, signal_rc);
        let res = process.ready();
        assert!(res.is_ok());

        let res = process.ready();
        assert!(res.is_err());
        match res {
            Ok(_) => unreachable!(),
            Err(ProcessError::ResultAlreadyGiven) => {},
            _ => assert!(false, "Wrong error type"),
        }

        // Finish workflow
        process.signal(Signal::INT);
        assert!(rc.recv().unwrap());
        assert!(process.wait().is_ok());
    }

    #[test]
    fn test_wait_and_signal_process() {
        let (sn, rc) = chan::sync(0);
        let runner = TestRunner::new(0, sn);

        let (signal_sn, signal_rc) = chan::sync(9);
        let process = MaridProcess::new(runner, signal_sn, signal_rc);
        process.signal(Signal::INT);

        assert!(rc.recv().unwrap()); // rendevous channel goes first
        let res = process.wait();
        assert!(res.is_ok());

        let res = process.wait();
        assert!(res.is_err());
        match res {
            Ok(_) => unreachable!(),
            Err(ProcessError::ResultAlreadyGiven) => {},
            _ => assert!(false, "Wrong error type"),
        }
    }
}