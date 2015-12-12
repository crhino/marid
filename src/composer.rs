use crossbeam;
use traits::{Runner, Signal, Receiver, Sender};
use {MaridError};
use chan;
use std::thread;
use std::sync::{Arc, Mutex};

/// The Composer type.
///
/// The Composer will start each runner inside of its own thread when the run() function
/// is called. The current behavior is an ordered setup/run, but in the future a parallel
/// startup mode will be offered.
pub struct Composer<R> {
    runners: Vec<R>,
    state: State,
    error_signal: Signal,
}

enum State {
    Init,
    SetupDone,
}

impl<R> Composer<R> {
    /// Creates a new Composer.
    ///
    /// The error_signal is the Signal that the Composer will send to
    /// runners when another runner in the group has finished with an error.
    pub fn new(runners: Vec<R>, error_signal: Signal) -> Composer<R> {
        Composer{
            runners: runners,
            state: State::Init,
            error_signal: error_signal,
        }
    }

    fn take_runners_and_setup_signal_chan(self) -> (Vec<(R, Receiver<Signal>)>, Vec<Sender<Signal>>) {
        let mut runners_vec = vec!();
        let mut sender_vec = vec!();
        for r in self.runners.into_iter() {
            let (sn, rc) = chan::sync(1024);
            runners_vec.push((r, rc));
            sender_vec.push(sn);
        }
        (runners_vec, sender_vec)
    }
}

impl Runner for Composer<Box<Runner + Send>> {
    fn run(mut self: Box<Self>, signals: Receiver<Signal>) -> Result<(), MaridError> {
        match self.state {
            State::Init => try!(self.setup()),
            _ => {},
        }

        let error_signal = self.error_signal;
        let (runners_vec, sender_vec) = self.take_runners_and_setup_signal_chan();
        let (stop_sn, stop_rc) = chan::sync(0);
        let (error_sn, error_rc) = chan::sync(1);
        let signaling = signaling_thread(signals,
                                         sender_vec,
                                         stop_rc,
                                         error_signal,
                                         error_rc);
        let error = Arc::new(Mutex::new(None));
        let error_clone = error.clone();

        crossbeam::scope(|scope| {
            for (r, rc) in runners_vec.into_iter() {
                let err = error_clone.clone();
                let err_sn = error_sn.clone();
                scope.spawn(move || {
                    match r.run(rc) {
                        Ok(()) => {},
                        Err(e) => {
                            let mut guard = err.lock().unwrap();
                            *guard = Some(e);
                            err_sn.send(true);
                        },
                    }
                });
            }
        });

        stop_sn.send(true);
        signaling.join().unwrap();
        take_error(error)
    }

    fn setup(&mut self) -> Result<(), MaridError> {
        for r in self.runners.iter_mut() {
            try!(r.setup());
        }
        self.state = State::SetupDone;
        Ok(())
    }
}

fn take_error(err: Arc<Mutex<Option<MaridError>>>) -> Result<(), MaridError> {
        match err.lock().unwrap().take() {
            Some(e) => Err(e),
            None => Ok(()),
        }
}

fn signaling_thread(signals: Receiver<Signal>,
                    senders: Vec<Sender<Signal>>,
                    quit: Receiver<bool>,
                    error_signal: Signal,
                    error: Receiver<bool>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            chan_select! {
                signals.recv() -> res => {
                    let sig = match res {
                        Some(s) => s,
                        None => continue,
                    };

                    for sn in senders.iter() {
                        sn.send(sig);
                    }
                },
                error.recv() => {
                    for sn in senders.iter() {
                        sn.send(error_signal);
                    }
                },
                quit.recv() => {
                    return
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use test_helpers::{TestRunner, TestError};
    use {Composer, Runner, Signal, MaridError};
    use thunk::Thunk;
    use chan;
    use std::thread;

    #[test]
    fn test_composer_runner() {
        let (sn, rc) = chan::sync(2);

        let runner1 = Box::new(TestRunner::new(1, sn.clone())) as Box<Runner + Send>;
        let runner2 = Box::new(TestRunner::new(2, sn)) as Box<Runner + Send>;

        let (sig_send, signals) = chan::async();

        let mut composer = Box::new(Composer::new(vec!(runner1, runner2), Signal::INT));
        let res = composer.setup();
        assert!(res.is_ok());

        thread::spawn(move || {
            sig_send.send(Signal::INT);
        });

        let res = composer.run(signals);
        assert!(res.is_ok());

        assert!(rc.recv().expect("Did not recv"));
        assert!(rc.recv().expect("Did not recv"));
    }

    #[test]
    fn test_composer_error() {
        let (sn, rc) = chan::sync(2);

        let runner1 = Box::new(TestRunner::new(1, sn.clone())) as Box<Runner + Send>;
        let runner2 = Box::new(TestRunner::new(2, sn)) as Box<Runner + Send>;

        let (sig_send, signals) = chan::async();

        let mut composer = Box::new(Composer::new(vec!(runner1, runner2), Signal::INT));
        let res = composer.setup();
        assert!(res.is_ok());

        thread::spawn(move || {
            sig_send.send(Signal::HUP);
        });

        let res = composer.run(signals);
        assert!(res.is_err());

        assert!(!rc.recv().expect("Did not recv"));
        assert!(!rc.recv().expect("Did not recv"));
    }

    #[test]
    fn test_composer_error_inside_runner() {
        let (sn, _rc) = chan::sync(2);

        let runner1 = Box::new(TestRunner::new(1, sn)) as Box<Runner + Send>;
        let runner2 = Box::new(Thunk::with_arg(move |_sigs| {
            Err(Box::new(TestError) as MaridError)
        })) as Box<Runner + Send>;

        let (_sig_send, signals) = chan::async();

        let mut composer = Box::new(Composer::new(vec!(runner1, runner2), Signal::INT));
        let res = composer.setup();
        assert!(res.is_ok());

        let res = composer.run(signals);
        assert!(res.is_err());
    }

    #[test]
    fn test_composer_run_no_setup() {
        let (sn, rc) = chan::sync(2);

        let runner1 = Box::new(TestRunner::new(1, sn.clone())) as Box<Runner + Send>;
        let runner2 = Box::new(TestRunner::new(2, sn)) as Box<Runner + Send>;

        let (sig_send, signals) = chan::async();

        let composer = Box::new(Composer::new(vec!(runner1, runner2), Signal::INT));
        thread::spawn(move || {
            sig_send.send(Signal::INT);
        });

        let res = composer.run(signals);
        assert!(res.is_ok());

        assert!(rc.recv().expect("Did not recv"));
        assert!(rc.recv().expect("Did not recv"));
    }
}
